#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::http::Response;
use tauri::Manager;
use tokio::sync::Mutex as TokioMutex;
use std::sync::Mutex as StdMutex;

mod constants;
mod db;
mod folders;
mod notes;
mod search;
mod workspace;
mod attachments;

// app memory
pub struct AppState {
    pub db: TokioMutex<Option<SqlitePool>>,
    pub workspace_path: StdMutex<Option<String>>, // Zmienione na standardowy Mutex!
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: TokioMutex::new(None),
            workspace_path: StdMutex::new(None),
        })
        // NASZ NIESTANDARDOWY BEZPIECZNY PROTOKÓŁ!
        .register_uri_scheme_protocol("accord", move |app, request| {
            let state = app.state::<AppState>();
            let ws_guard = state.workspace_path.lock().unwrap();
            
            if let Some(ws) = ws_guard.as_ref() {
                // Odczytujemy URL, np. accord://local/123.png -> local/123.png
                let uri = request.uri().path().trim_start_matches('/');
                
                let file_path = if uri.starts_with("local/") {
                    // Bezpiecznie łączymy z folderem workspace/attachments
                    let file_name = uri.trim_start_matches("local/");
                    Path::new(ws).join(constants::ATTACHMENTS_DIR).join(file_name)
                } else if uri.starts_with("link/") {
                    // Dekodujemy absolutną ścieżkę z systemu
                    let encoded_path = uri.trim_start_matches("link/");
                    let decoded = urlencoding::decode(encoded_path).unwrap_or_default();
                    PathBuf::from(decoded.into_owned())
                } else {
                    return Response::builder().status(400).body(vec![]).unwrap();
                };

                if let Ok(data) = fs::read(&file_path) {
                    return Response::builder()
                        .status(200)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(data)
                        .unwrap();
                }
            }
            Response::builder().status(404).body(vec![]).unwrap()
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
            attachments::attach_file_copy, 
            attachments::attach_file_move, 
            attachments::attach_file_link, 
            attachments::attach_blob
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
