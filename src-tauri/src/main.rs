#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod workspace;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            workspace::get_workspace,
            workspace::set_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}