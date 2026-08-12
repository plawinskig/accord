use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use crate::AppState;

// folder structure (database folders) 
#[derive(Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}

// select all folders not in the trash
#[tauri::command]
pub async fn get_folders(state: State<'_, AppState>) -> Result<Vec<Folder>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    // sort alphabetically
    let folders = sqlx::query_as!(
        Folder,
        r#"
        SELECT 
            id as "id!", 
            parent_id, 
            name as "name!" 
        FROM folders 
        WHERE is_deleted = 0 
        ORDER BY name ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(folders)
}

// create a new folder with a secure UUID
#[tauri::command]
pub async fn create_folder(name: String, parent_id: Option<String>, state: State<'_, AppState>) -> Result<Folder, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO folders (id, parent_id, name, is_deleted) VALUES (?, ?, ?, 0)",
        id, parent_id, name
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // return the created folder to the interface so that it is displayed immediately
    Ok(Folder { id, parent_id, name })
}