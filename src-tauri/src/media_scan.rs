use std::{
    cmp::Ordering,
    collections::HashMap,
    fs,
    fs::File,
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::db;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRecord {
    id: i64,
    file_name: String,
    file_path: String,
    source_path: Option<String>,
    source_url: Option<String>,
    source_title: Option<String>,
    file_type: String,
    mime_type: Option<String>,
    file_size: Option<i64>,
    thumbnail_url: Option<String>,
    imported_at: String,
    tag_ids: Vec<i64>,
    page_count: Option<i64>,
    current_page_index: Option<i64>,
    series_name: Option<String>,
    completed_at: Option<String>,
    last_viewed_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMediaResult {
    folder_path: String,
    total_scanned: usize,
    total_imported: usize,
    media_items: Vec<MediaRecord>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCategoryRecord {
    id: i64,
    name: String,
    color: Option<String>,
    sort_order: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRecord {
    id: i64,
    category_id: i64,
    name: String,
    description: Option<String>,
    color: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    media_items: Vec<MediaRecord>,
    tag_categories: Vec<TagCategoryRecord>,
    tags: Vec<TagRecord>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicPagesResponse {
    media_id: i64,
    page_paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMetadataResponse {
    final_url: String,
    title: String,
    media_item: MediaRecord,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComicMetadata {
    page_count: usize,
    page_paths: Vec<String>,
    #[serde(default)]
    current_page_index: usize,
}

const MAX_COMIC_PAGE_COUNT: usize = 5000;
const MAX_COMIC_ENTRY_BYTES: u64 = 100 * 1024 * 1024;
const MAX_COMIC_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[tauri::command(rename_all = "camelCase")]
pub fn load_media_items(app: AppHandle) -> Result<Vec<MediaRecord>, String> {
    let connection = db::connect(&app).map_err(|error| error.to_string())?;
    list_media_items(&connection).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn load_library_snapshot(app: AppHandle) -> Result<LibrarySnapshot, String> {
    let connection = db::connect(&app).map_err(|error| error.to_string())?;
    build_library_snapshot(&connection).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_media_source_from_url(
    app: AppHandle,
    media_id: i64,
    source_url: String,
) -> Result<SourceMetadataResponse, String> {
    let normalized_url = normalized_required(source_url, "URLを入力してください。")?;

    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let media_type: Option<String> = transaction
        .query_row(
            "SELECT file_type FROM media WHERE id = ?1 LIMIT 1",
            [media_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let Some(media_type) = media_type else {
        return Err("指定されたメディアが存在しません。".to_string());
    };

    if media_type != "comic" {
        return Err("元URLからのタイトル取得は漫画ZIP専用です。".to_string());
    }

    let (final_url, title) = fetch_url_title(&normalized_url)?;

    transaction
        .execute(
            "UPDATE media SET source_url = ?1, source_title = ?2 WHERE id = ?3",
            params![final_url, title, media_id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    let media_item = find_media_item(&connection, media_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "更新後のメディアを読み込めませんでした。".to_string())?;

    Ok(SourceMetadataResponse {
        final_url,
        title,
        media_item,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn load_comic_pages(app: AppHandle, media_id: i64) -> Result<ComicPagesResponse, String> {
    let connection = db::connect(&app).map_err(|error| error.to_string())?;

    let (file_type, file_path, metadata_json): (String, String, Option<String>) = connection
        .query_row(
            "SELECT file_type, file_path, metadata_json FROM media WHERE id = ?1 LIMIT 1",
            [media_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| error.to_string())?;

    if file_type != "comic" {
        return Err("指定されたメディアは漫画ZIPではありません。".to_string());
    }

    let Some(metadata) = parse_comic_metadata(metadata_json.as_deref()) else {
        return Err("漫画ページ情報を読み込めませんでした。".to_string());
    };

    let archive_directory = PathBuf::from(file_path)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "漫画管理ディレクトリを解決できませんでした。".to_string())?;

    Ok(ComicPagesResponse {
        media_id,
        page_paths: metadata
            .page_paths
            .into_iter()
            .map(|relative_path| archive_directory.join(relative_path).to_string_lossy().to_string())
            .collect(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_comic_progress(app: AppHandle, media_id: i64, page_index: usize) -> Result<MediaRecord, String> {
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;

    let (file_type, metadata_json): (String, Option<String>) = transaction
        .query_row(
            "SELECT file_type, metadata_json FROM media WHERE id = ?1 LIMIT 1",
            [media_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;

    if file_type != "comic" {
        return Err("読了位置の保存は漫画ZIP専用です。".to_string());
    }

    let Some(mut metadata) = parse_comic_metadata(metadata_json.as_deref()) else {
        return Err("漫画ページ情報を読み込めませんでした。".to_string());
    };

    if metadata.page_count == 0 {
        return Err("漫画ページ数が不正です。".to_string());
    }

    metadata.current_page_index = page_index.min(metadata.page_count.saturating_sub(1));
    let next_metadata_json = serde_json::to_string(&metadata).map_err(|error| error.to_string())?;

    let now = current_timestamp_string();
    let is_last_page: i64 = if page_index >= metadata.page_count.saturating_sub(1) {
        1
    } else {
        0
    };

    transaction
        .execute(
            "UPDATE media SET metadata_json = ?1, last_viewed_at = ?2, completed_at = CASE WHEN ?3 = 1 THEN ?2 ELSE completed_at END WHERE id = ?4",
            params![next_metadata_json, now, is_last_page, media_id],
        )
        .map_err(|error| error.to_string())?;

    transaction.commit().map_err(|error| error.to_string())?;

    find_media_item(&connection, media_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "更新後のメディアを読み込めませんでした。".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_media_series(app: AppHandle, media_id: i64, series_name: String) -> Result<MediaRecord, String> {
    let normalized = normalized_required(series_name, "シリーズ名を入力してください。")?;

    let connection = db::connect(&app).map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE media SET series_name = ?1 WHERE id = ?2",
            params![normalized, media_id],
        )
        .map_err(|error| error.to_string())?;

    find_media_item(&connection, media_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "更新後のメディアを読み込めませんでした。".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn touch_last_viewed(app: AppHandle, media_id: i64) -> Result<MediaRecord, String> {
    let connection = db::connect(&app).map_err(|error| error.to_string())?;
    let now = current_timestamp_string();
    connection
        .execute(
            "UPDATE media SET last_viewed_at = ?1 WHERE id = ?2",
            params![now, media_id],
        )
        .map_err(|error| error.to_string())?;

    find_media_item(&connection, media_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "更新後のメディアを読み込めませんでした。".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn import_media_folder(app: AppHandle, folder_path: String) -> Result<ImportMediaResult, String> {
    let root = PathBuf::from(&folder_path);
    if !root.is_dir() {
        return Err("選択されたパスはフォルダではありません。".to_string());
    }

    let canonical_root = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    let managed_media_root = db::managed_media_root(&app).map_err(|error| error.to_string())?;
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let imported_at = current_timestamp_string();
    let import_directory = managed_media_root.join(format!(
        "{}__{}",
        current_timestamp_nonce_string(),
        sanitize_path_segment(
            canonical_root
                .file_name()
                .and_then(|segment| segment.to_str())
                .unwrap_or("library")
        )
    ));
    fs::create_dir_all(&import_directory).map_err(|error| error.to_string())?;

    let mut total_scanned = 0_usize;
    let mut total_imported = 0_usize;

    let import_result: Result<(), String> = (|| {
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            let Some(file_type) = detect_media_type(path) else {
                continue;
            };

            let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            if file_type == "comic" {
                match archive_contains_supported_images(&canonical_path) {
                    Ok(true) => {}
                    Ok(false) | Err(_) => continue,
                }
            }

            total_scanned += 1;

            total_imported += insert_media_record(
                &transaction,
                &canonical_root,
                &canonical_path,
                &import_directory,
                &imported_at,
                file_type,
            )?;
        }

        Ok(())
    })();

    if let Err(error) = import_result {
        let _ = fs::remove_dir_all(&import_directory);
        return Err(error);
    }

    if let Err(error) = transaction.commit() {
        let _ = fs::remove_dir_all(&import_directory);
        return Err(error.to_string());
    }

    if total_imported == 0 {
        let _ = fs::remove_dir_all(&import_directory);
    }

    let media_items = list_media_items(&connection).map_err(|error| error.to_string())?;

    Ok(ImportMediaResult {
        folder_path,
        total_scanned,
        total_imported,
        media_items,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn import_media_file(app: AppHandle, file_path: String) -> Result<ImportMediaResult, String> {
    let path = PathBuf::from(&file_path);
    if !path.is_file() {
        return Err("選択されたパスはファイルではありません。".to_string());
    }

    let canonical_path = fs::canonicalize(&path).unwrap_or(path.clone());
    let Some(file_type) = detect_media_type(&canonical_path) else {
        return Err("対応している画像または動画ファイルを選択してください。".to_string());
    };

    if file_type == "comic" {
        return Err("単一ファイル取り込みは画像・動画専用です。漫画はフォルダ取り込みを利用してください。".to_string());
    }

    let managed_media_root = db::managed_media_root(&app).map_err(|error| error.to_string())?;
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let imported_at = current_timestamp_string();
    let import_directory = managed_media_root.join(format!(
        "{}__{}",
        current_timestamp_nonce_string(),
        sanitize_path_segment(
            canonical_path
                .file_stem()
                .and_then(|segment| segment.to_str())
                .unwrap_or("media_file")
        )
    ));
    fs::create_dir_all(&import_directory).map_err(|error| error.to_string())?;

    let total_imported = insert_media_record(
        &transaction,
        &canonical_path,
        &canonical_path,
        &import_directory,
        &imported_at,
        file_type,
    )?;

    if let Err(error) = transaction.commit() {
        let _ = fs::remove_dir_all(&import_directory);
        return Err(error.to_string());
    }

    if total_imported == 0 {
        let _ = fs::remove_dir_all(&import_directory);
    }

    let media_items = list_media_items(&connection).map_err(|error| error.to_string())?;

    Ok(ImportMediaResult {
        folder_path: file_path,
        total_scanned: 1,
        total_imported,
        media_items,
    })
}

fn insert_media_record(
    transaction: &rusqlite::Transaction<'_>,
    root: &Path,
    canonical_path: &Path,
    import_directory: &Path,
    imported_at: &str,
    file_type: &str,
) -> Result<usize, String> {
    let source_path = canonical_path.to_string_lossy().to_string();
    let already_imported: bool = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM media WHERE source_path = ?1 OR (source_path IS NULL AND file_path = ?1)
            )",
            [&source_path],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if already_imported {
        return Ok(0);
    }

    let metadata = fs::metadata(canonical_path).map_err(|error| error.to_string())?;
    let file_size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
    let mime_type = guess_mime_type(canonical_path);
    let managed_file_path =
        copy_into_managed_library(root, canonical_path, import_directory).map_err(|error| error.to_string())?;
    let file_path = managed_file_path.to_string_lossy().to_string();
    let (thumbnail_path, metadata_json) = if file_type == "comic" {
        import_comic_archive(&managed_file_path, import_directory).map_err(|error| error.to_string())?
    } else {
        (None, None)
    };

    transaction
        .execute(
            "INSERT INTO media (
                file_path,
                source_path,
                source_url,
                source_title,
                file_name,
                file_type,
                mime_type,
                width,
                height,
                duration_seconds,
                file_size,
                file_hash,
                thumbnail_path,
                imported_at,
                created_at,
                metadata_json,
                series_name,
                completed_at,
                last_viewed_at
            ) VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, NULL, NULL, NULL, ?6, NULL, ?7, ?8, NULL, ?9, NULL, NULL, NULL)",
            params![
                file_path,
                source_path,
                canonical_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("file"),
                file_type,
                mime_type,
                file_size,
                thumbnail_path,
                imported_at,
                metadata_json
            ],
        )
        .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn create_tag_category(
    app: AppHandle,
    name: String,
    color: Option<String>,
) -> Result<TagCategoryRecord, String> {
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let name = normalized_required(name, "カテゴリ名を入力してください。")?;
    let color = normalized_optional(color);

    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let next_sort_order: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM tag_categories",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    transaction
        .execute(
            "INSERT INTO tag_categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
            params![name, color, next_sort_order],
        )
        .map_err(|error| map_constraint_error(error, "同名のカテゴリがすでに存在します。"))?;

    let created_id = transaction.last_insert_rowid();
    transaction.commit().map_err(|error| error.to_string())?;

    find_tag_category(&connection, created_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "作成したカテゴリを読み込めませんでした。".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn create_tag(
    app: AppHandle,
    category_id: i64,
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> Result<TagRecord, String> {
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let name = normalized_required(name, "タグ名を入力してください。")?;
    let description = normalized_optional(description);
    let color = normalized_optional(color);

    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let category_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tag_categories WHERE id = ?1)",
            [category_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if !category_exists {
        return Err("指定されたカテゴリが存在しません。".to_string());
    }

    transaction
        .execute(
            "INSERT INTO tags (category_id, name, description, color) VALUES (?1, ?2, ?3, ?4)",
            params![category_id, name, description, color],
        )
        .map_err(|error| map_constraint_error(error, "同じカテゴリに同名のタグがすでに存在します。"))?;

    let created_id = transaction.last_insert_rowid();
    transaction.commit().map_err(|error| error.to_string())?;

    find_tag(&connection, created_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "作成したタグを読み込めませんでした。".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_media_tags(app: AppHandle, media_id: i64, tag_ids: Vec<i64>) -> Result<MediaRecord, String> {
    let mut connection = db::connect(&app).map_err(|error| error.to_string())?;
    let mut tag_ids = tag_ids;
    tag_ids.sort_unstable();
    tag_ids.dedup();

    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let media_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM media WHERE id = ?1)",
            [media_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if !media_exists {
        return Err("指定されたメディアが存在しません。".to_string());
    }

    transaction
        .execute("DELETE FROM media_tags WHERE media_id = ?1", [media_id])
        .map_err(|error| error.to_string())?;

    for tag_id in &tag_ids {
        transaction
            .execute(
                "INSERT INTO media_tags (media_id, tag_id) VALUES (?1, ?2)",
                params![media_id, tag_id],
            )
            .map_err(|error| map_constraint_error(error, "存在しないタグは割り当てできません。"))?;
    }

    transaction.commit().map_err(|error| error.to_string())?;

    find_media_item(&connection, media_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "更新後のメディアを読み込めませんでした。".to_string())
}

fn build_library_snapshot(connection: &Connection) -> Result<LibrarySnapshot, rusqlite::Error> {
    Ok(LibrarySnapshot {
        media_items: list_media_items(connection)?,
        tag_categories: list_tag_categories(connection)?,
        tags: list_tags(connection)?,
    })
}

fn list_media_items(connection: &Connection) -> Result<Vec<MediaRecord>, rusqlite::Error> {
    let media_tag_map = load_media_tag_map(connection)?;
    let mut statement = connection.prepare(
        "SELECT id, file_name, file_path, source_path, source_url, source_title, file_type, mime_type, file_size, thumbnail_path, imported_at, metadata_json, series_name, completed_at, last_viewed_at
         FROM media
         ORDER BY imported_at DESC, id DESC",
    )?;

    let rows = statement.query_map([], |row| {
        let media_id: i64 = row.get(0)?;
        let metadata_json: Option<String> = row.get(11)?;
        let comic_metadata = parse_comic_metadata(metadata_json.as_deref());

        Ok(MediaRecord {
            id: media_id,
            file_name: row.get(1)?,
            file_path: row.get(2)?,
            source_path: row.get(3)?,
            source_url: row.get(4)?,
            source_title: row.get(5)?,
            file_type: row.get(6)?,
            mime_type: row.get(7)?,
            file_size: row.get(8)?,
            thumbnail_url: row.get(9)?,
            imported_at: row.get(10)?,
            tag_ids: media_tag_map.get(&media_id).cloned().unwrap_or_default(),
            page_count: comic_metadata.as_ref().map(|metadata| metadata.page_count as i64),
            current_page_index: comic_metadata.as_ref().map(|metadata| metadata.current_page_index as i64),
            series_name: row.get(12)?,
            completed_at: row.get(13)?,
            last_viewed_at: row.get(14)?,
        })
    })?;

    rows.collect()
}

fn find_media_item(connection: &Connection, media_id: i64) -> Result<Option<MediaRecord>, rusqlite::Error> {
    Ok(list_media_items(connection)?
        .into_iter()
        .find(|media_item| media_item.id == media_id))
}

fn load_media_tag_map(connection: &Connection) -> Result<HashMap<i64, Vec<i64>>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT media_id, tag_id FROM media_tags ORDER BY media_id ASC, tag_id ASC",
    )?;

    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    let mut map = HashMap::<i64, Vec<i64>>::new();

    for row in rows {
        let (media_id, tag_id) = row?;
        map.entry(media_id).or_default().push(tag_id);
    }

    Ok(map)
}

fn list_tag_categories(connection: &Connection) -> Result<Vec<TagCategoryRecord>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT id, name, color, sort_order FROM tag_categories ORDER BY sort_order ASC, id ASC",
    )?;

    let rows = statement.query_map([], |row| {
        Ok(TagCategoryRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;

    rows.collect()
}

fn find_tag_category(
    connection: &Connection,
    category_id: i64,
) -> Result<Option<TagCategoryRecord>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT id, name, color, sort_order FROM tag_categories WHERE id = ?1 LIMIT 1",
    )?;

    let mut rows = statement.query([category_id])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(TagCategoryRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            sort_order: row.get(3)?,
        }));
    }

    Ok(None)
}

fn list_tags(connection: &Connection) -> Result<Vec<TagRecord>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT t.id, t.category_id, t.name, t.description, t.color
         FROM tags t
         JOIN tag_categories c ON c.id = t.category_id
         ORDER BY c.sort_order ASC, t.name COLLATE NOCASE ASC, t.id ASC",
    )?;

    let rows = statement.query_map([], |row| {
        Ok(TagRecord {
            id: row.get(0)?,
            category_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            color: row.get(4)?,
        })
    })?;

    rows.collect()
}

fn find_tag(connection: &Connection, tag_id: i64) -> Result<Option<TagRecord>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT id, category_id, name, description, color FROM tags WHERE id = ?1 LIMIT 1",
    )?;

    let mut rows = statement.query([tag_id])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(TagRecord {
            id: row.get(0)?,
            category_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            color: row.get(4)?,
        }));
    }

    Ok(None)
}

fn normalized_required(value: String, error_message: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(error_message.to_string());
    }

    Ok(trimmed.to_string())
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value.and_then(|content| {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn map_constraint_error(error: rusqlite::Error, fallback: &str) -> String {
    match error {
        rusqlite::Error::SqliteFailure(inner, Some(message)) => {
            if inner.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                fallback.to_string()
            } else {
                message
            }
        }
        rusqlite::Error::SqliteFailure(inner, None) => {
            if inner.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                fallback.to_string()
            } else {
                fallback.to_string()
            }
        }
        _ => fallback.to_string(),
    }
}

fn copy_into_managed_library(
    root: &Path,
    source_path: &Path,
    import_directory: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let relative_path = source_path
        .strip_prefix(root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            source_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("file"))
        });

    let destination_path = unique_destination_path(import_directory.join(relative_path));
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source_path, &destination_path)?;

    Ok(destination_path)
}

fn import_comic_archive(
    archive_path: &Path,
    import_directory: &Path,
) -> Result<(Option<String>, Option<String>), Box<dyn std::error::Error>> {
    let archive_file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(archive_file)?;
    let archive_directory = archive_path.parent().unwrap_or(import_directory);
    let archive_identifier = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(sanitize_file_name)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "comic_zip".to_string());
    let extraction_root = archive_directory.join("__comic_pages").join(format!("{}_pages", archive_identifier));
    fs::create_dir_all(&extraction_root)?;

    let mut page_paths = Vec::new();
    let mut image_entries = Vec::<(String, usize)>::new();
    let mut total_uncompressed_bytes = 0_u64;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let Some(enclosed_name) = entry.enclosed_name().map(Path::to_path_buf) else {
            continue;
        };
        if !is_supported_image_path(&enclosed_name) {
            continue;
        }

        total_uncompressed_bytes = total_uncompressed_bytes.saturating_add(entry.size());
        if entry.size() > MAX_COMIC_ENTRY_BYTES {
            let _ = fs::remove_dir_all(&extraction_root);
            return Err("漫画ZIP内に大きすぎるページ画像が含まれています。".into());
        }
        if total_uncompressed_bytes > MAX_COMIC_TOTAL_BYTES {
            let _ = fs::remove_dir_all(&extraction_root);
            return Err("漫画ZIPの展開サイズが大きすぎます。".into());
        }

        image_entries.push((enclosed_name.to_string_lossy().to_string(), index));
    }

    if image_entries.len() > MAX_COMIC_PAGE_COUNT {
        let _ = fs::remove_dir_all(&extraction_root);
        return Err("漫画ZIPのページ数が多すぎます。".into());
    }

    image_entries.sort_by(|(left, _), (right, _)| natural_path_compare(left, right));

    for (page_number, (entry_name, entry_index)) in image_entries.into_iter().enumerate() {
        let mut entry = archive.by_index(entry_index)?;
        let enclosed_name = PathBuf::from(entry_name);

        let entry_file_name = enclosed_name
            .file_name()
            .and_then(|value| value.to_str())
            .map(sanitize_file_name)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("page_{entry_index:04}.img"));

        let destination_path = unique_destination_path(
            extraction_root.join(format!("{:04}_{}", page_number + 1, entry_file_name)),
        );

        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = File::create(&destination_path)?;
        io::copy(&mut entry, &mut output)?;
        let relative_page_path = destination_path
            .strip_prefix(archive_directory)
            .map(Path::to_path_buf)
            .unwrap_or(destination_path.clone());
        page_paths.push(relative_page_path.to_string_lossy().to_string());
    }

    if page_paths.is_empty() {
        let _ = fs::remove_dir_all(&extraction_root);
        return Err("漫画ZIP内に画像ページが見つかりませんでした。".into());
    }

    let metadata_json = serde_json::to_string(&ComicMetadata {
        page_count: page_paths.len(),
        page_paths: page_paths.clone(),
        current_page_index: 0,
    })?;

    let thumbnail_path = page_paths
        .first()
        .map(|relative_path| archive_directory.join(relative_path).to_string_lossy().to_string());

    Ok((thumbnail_path, Some(metadata_json)))
}

fn unique_destination_path(destination_path: PathBuf) -> PathBuf {
    if !destination_path.exists() {
        return destination_path;
    }

    let parent = destination_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = destination_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = destination_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_string);

    for index in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_{index}.{extension}"),
            None => format!("{stem}_{index}"),
        };

        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    destination_path
}

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "avif"))
        .unwrap_or(false)
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();

    sanitized.trim_matches('_').to_string()
}

fn natural_path_compare(left: &str, right: &str) -> Ordering {
    let left_parts = split_natural_parts(left);
    let right_parts = split_natural_parts(right);

    for (left_part, right_part) in left_parts.iter().zip(right_parts.iter()) {
        let ordering = match (left_part.parse::<u64>(), right_part.parse::<u64>()) {
            (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
            _ => left_part.to_ascii_lowercase().cmp(&right_part.to_ascii_lowercase()),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    left_parts.len().cmp(&right_parts.len())
}

fn split_natural_parts(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_is_digit = None;

    for character in value.chars() {
        let is_digit = character.is_ascii_digit();
        match current_is_digit {
            Some(existing_is_digit) if existing_is_digit == is_digit => current.push(character),
            Some(_) => {
                parts.push(current);
                current = String::from(character);
                current_is_digit = Some(is_digit);
            }
            None => {
                current.push(character);
                current_is_digit = Some(is_digit);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "library".to_string()
    } else {
        sanitized
    }
}

fn parse_comic_metadata(metadata_json: Option<&str>) -> Option<ComicMetadata> {
    metadata_json.and_then(|value| serde_json::from_str::<ComicMetadata>(value).ok())
}

fn fetch_url_title(source_url: &str) -> Result<(String, String), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Media Shelf/0.1")
        .build()
        .map_err(|error| error.to_string())?;

    let response = client
        .get(source_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("URLの取得に失敗しました: {error}"))?;

    let final_url = response.url().to_string();
    let body = response
        .text()
        .map_err(|error| format!("HTMLの読み込みに失敗しました: {error}"))?;

    let document = Html::parse_document(&body);
    let meta_selector = Selector::parse("meta[property='og:title'], meta[name='twitter:title']")
        .map_err(|error| error.to_string())?;
    let title_selector = Selector::parse("title").map_err(|error| error.to_string())?;

    let title = document
        .select(&meta_selector)
        .filter_map(|element| element.value().attr("content"))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            document
                .select(&title_selector)
                .next()
                .map(|element| element.text().collect::<String>())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .ok_or_else(|| "タイトルを抽出できませんでした。".to_string())?;

    Ok((final_url, title))
}

fn archive_contains_supported_images(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let archive_file = File::open(path)?;
    let mut archive = ZipArchive::new(archive_file)?;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        if let Some(enclosed_name) = entry.enclosed_name() {
            if is_supported_image_path(enclosed_name) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn detect_media_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "avif"
        | "heic" => Some("image"),
        "mp4" | "mov" | "m4v" | "webm" | "mkv" | "avi" => Some("video"),
        "zip" | "cbz" => Some("comic"),
        _ => None,
    }
}

fn guess_mime_type(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    let mime = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "avif" => "image/avif",
        "heic" => "image/heic",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "zip" => "application/zip",
        "cbz" => "application/vnd.comicbook+zip",
        _ => return None,
    };

    Some(mime.to_string())
}

fn current_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn current_timestamp_nonce_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
