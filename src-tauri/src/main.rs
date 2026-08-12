#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sqlx::SqlitePool;
use tokio::sync::Mutex;

mod workspace;
mod db;
mod folders;

// app memory
pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            workspace::get_workspace,
            workspace::set_workspace,
            db::connect_to_db,
            folders::get_folders,
            folders::create_folder,
            folders::soft_delete_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}