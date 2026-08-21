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
    pub workspace_path: StdMutex<Option<String>>,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: TokioMutex::new(None),
            workspace_path: StdMutex::new(None),
        })
        .register_uri_scheme_protocol("accord", move |app, request| {
            let state = app.app_handle().state::<AppState>();
            let ws_guard = state.workspace_path.lock().unwrap();
            
            if let Some(ws) = ws_guard.as_ref() {
                
                // 1. Zamiast dzielić na host i path, bierzemy po prostu cały surowy URL
                let uri_str = request.uri().to_string();
                
                // Wypiszemy to do terminala, żeby w razie problemów widzieć co się dzieje!
                println!("[Accord Protocol] Otrzymano żądanie: {}", uri_str);
                
                // 2. Szukamy naszych słów kluczowych niezależnie od tego, jak Tauri sparsowało link
                let file_path = if let Some((_, local_part)) = uri_str.split_once("accord://local/") {
                    
                    let decoded = urlencoding::decode(local_part).unwrap_or_default();
                    Path::new(ws).join(constants::ATTACHMENTS_DIR).join(decoded.into_owned())
                    
                } else if let Some((_, link_part)) = uri_str.split_once("accord://link/") {
                    
                    let decoded = urlencoding::decode(link_part).unwrap_or_default();
                    PathBuf::from(decoded.into_owned())
                    
                } else if let Some((_, local_part)) = uri_str.split_once("/local/") { 
                    
                    // Fallback jeśli Tauri wstrzyknęło np. "localhost" w środek linku
                    let decoded = urlencoding::decode(local_part).unwrap_or_default();
                    Path::new(ws).join(constants::ATTACHMENTS_DIR).join(decoded.into_owned())
                    
                } else {
                    println!("[Accord Protocol] Nierozpoznany format URL!");
                    return Response::builder().status(400).body(vec![]).unwrap();
                };

                println!("[Accord Protocol] Szukam pliku na dysku: {:?}", file_path);

                // 3. Odczytujemy plik i nadajemy mu odpowiedni typ (MIME), by przeglądarka umiała go narysować
                if let Ok(data) = fs::read(&file_path) {
                    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let mime_type = match extension.to_lowercase().as_str() {
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "webp" => "image/webp",
                        "pdf" => "application/pdf",
                        _ => "application/octet-stream",
                    };

                    println!("[Accord Protocol] Sukces! Wysyłam jako {}", mime_type);

                    return Response::builder()
                        .status(200)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Content-Type", mime_type)
                        .body(data)
                        .unwrap();
                } else {
                    println!("[Accord Protocol] BŁĄD: Nie znaleziono pliku na dysku!");
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
