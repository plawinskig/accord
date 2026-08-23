use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// Define the database folder structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}

// Select all folders that are not in the trash
#[tauri::command]
pub async fn get_folders(state: State<'_, AppState>) -> Result<Vec<Folder>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    // Sort the folders alphabetically
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

// Create a new folder with a secure UUID
#[tauri::command]
pub async fn create_folder(
    name: String,
    parent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Folder, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO folders (id, parent_id, name, is_deleted) VALUES (?, ?, ?, 0)",
        id,
        parent_id,
        name
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Return the created folder so the interface can display it immediately
    Ok(Folder {
        id,
        parent_id,
        name,
    })
}

#[tauri::command]
pub async fn soft_delete_folder(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query!("UPDATE folders SET is_deleted = 1 WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
