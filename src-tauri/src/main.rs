#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sqlx::SqlitePool;
use tokio::sync::Mutex;

mod workspace;
mod db;
mod folders;
mod notes;
mod search;
mod attachments;
mod constants;

// app memory
pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub workspace_path: Mutex<Option<String>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: Mutex::new(None),
            workspace_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            workspace::get_workspace,
            workspace::set_workspace,
            db::connect_to_db,
            folders::get_folders,
            folders::create_folder,
            folders::soft_delete_folder,
            notes::get_notes,
            notes::create_note,
            notes::update_note,
            notes::soft_delete_note,
            search::search_notes,
            attachments::attach_file_copy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}