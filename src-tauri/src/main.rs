#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod media_scan;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            media_scan::create_tag,
            media_scan::create_tag_category,
            media_scan::import_media_file,
            media_scan::import_media_folder,
            media_scan::load_comic_pages,
            media_scan::load_library_snapshot,
            media_scan::load_media_items,
            media_scan::set_comic_progress,
            media_scan::set_media_series,
            media_scan::set_media_source_from_url,
            media_scan::set_media_tags,
            media_scan::touch_last_viewed
        ])
        .setup(|app| {
            db::initialize(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Media Shelf");
}
