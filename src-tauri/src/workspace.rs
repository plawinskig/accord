use crate::constants::{APP_NAME, APP_ORG, APP_QUALIFIER, ATTACHMENTS_DIR};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// structure to save as a JSON file in the system
#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub workspace_path: String,
}

// finds the system settings folder (~/.config/accord on Linux)
pub fn get_config_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME) {
        let config_dir = proj_dirs.config_dir();
        // if the settings folder does not exist create it
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        Some(config_dir.join("config.json"))
    } else {
        None
    }
}

// frontend asks if we already have a workspace set
#[tauri::command]
pub fn get_workspace() -> Option<String> {
    let path = get_config_path()?;
    if let Ok(config_data) = fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&config_data) {
            return Some(config.workspace_path);
        }
    }
    None
}

// frontend sends the user-selected path
#[tauri::command]
pub fn set_workspace(path: String) -> Result<(), String> {
    let config_path = get_config_path().ok_or("Could not find system config directory.")?;

    // create a main workspace folder and a subfolder for attachments
    let workspace_dir = PathBuf::from(&path);
    fs::create_dir_all(&workspace_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(workspace_dir.join(ATTACHMENTS_DIR)).map_err(|e| e.to_string())?;

    // save selection in the config.json file
    let config = AppConfig {
        workspace_path: path,
    };
    let config_json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(config_path, config_json).map_err(|e| e.to_string())?;

    Ok(())
}
