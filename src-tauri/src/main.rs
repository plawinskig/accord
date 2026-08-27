#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sqlx::SqlitePool;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as TokioMutex;

mod attachments;
mod clipboard;
mod constants;
mod db;
mod folders;
mod notes;
mod protocol;
mod search;
mod tags;
mod workspace;

// Store application state
pub struct AppState {
    pub db: TokioMutex<Option<SqlitePool>>,
    pub workspace_path: StdMutex<Option<String>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: TokioMutex::new(None),
            workspace_path: StdMutex::new(None),
        })
        .register_uri_scheme_protocol("accord", protocol::handle_accord_protocol)
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
            attachments::attach_file_copy,
            attachments::attach_file_move,
            attachments::attach_file_link,
            attachments::attach_blob,
            clipboard::read_clipboard_image,
            tags::get_all_tags,
            tags::attach_tag,
            tags::detach_tag,
            tags::rename_tag,
            tags::delete_tag
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
