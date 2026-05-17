use std::{fs, path::PathBuf};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

const DB_FILE_NAME: &str = "media-shelf.sqlite3";
const MANAGED_MEDIA_DIR_NAME: &str = "managed-media";
const SCHEMA_SQL: &str = include_str!("../../database/schema.sql");
const DEFAULT_TAG_CATEGORIES: [(&str, &str, i64); 6] = [
    ("人物", "#ff8a80", 1),
    ("場所", "#82b1ff", 2),
    ("作品", "#b388ff", 3),
    ("状態", "#8c9eff", 4),
    ("用途", "#a7ffeb", 5),
    ("評価", "#ffd180", 6),
];

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join(DB_FILE_NAME);
    let connection = Connection::open(&db_path)?;

    connection.execute_batch(SCHEMA_SQL)?;
    migrate_media_table(&connection)?;
    seed_default_tag_categories(&connection)?;

    println!("Initialized SQLite database at {}", db_path.display());

    Ok(())
}

pub fn connect(app: &AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    let db_path = database_path(app)?;
    let connection = Connection::open(db_path)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(connection)
}

pub fn managed_media_root(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    let managed_media_dir = app_data_dir.join(MANAGED_MEDIA_DIR_NAME);
    fs::create_dir_all(&managed_media_dir)?;
    Ok(managed_media_dir)
}

fn database_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join(DB_FILE_NAME))
}

fn seed_default_tag_categories(connection: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    for (name, color, sort_order) in DEFAULT_TAG_CATEGORIES {
        connection.execute(
            "INSERT OR IGNORE INTO tag_categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
            (name, color, sort_order),
        )?;
    }

    Ok(())
}

fn migrate_media_table(connection: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut column_names = Vec::new();

    let mut statement = connection.prepare("PRAGMA table_info(media)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        column_names.push(column?);
    }

    if !column_names.iter().any(|column| column == "source_path") {
        connection.execute("ALTER TABLE media ADD COLUMN source_path TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "source_url") {
        connection.execute("ALTER TABLE media ADD COLUMN source_url TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "source_title") {
        connection.execute("ALTER TABLE media ADD COLUMN source_title TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "thumbnail_path") {
        connection.execute("ALTER TABLE media ADD COLUMN thumbnail_path TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "metadata_json") {
        connection.execute("ALTER TABLE media ADD COLUMN metadata_json TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "series_name") {
        connection.execute("ALTER TABLE media ADD COLUMN series_name TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "completed_at") {
        connection.execute("ALTER TABLE media ADD COLUMN completed_at TEXT", [])?;
    }

    if !column_names.iter().any(|column| column == "last_viewed_at") {
        connection.execute("ALTER TABLE media ADD COLUMN last_viewed_at TEXT", [])?;
    }

    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_source_path ON media(source_path)",
        [],
    )?;

    Ok(())
}
