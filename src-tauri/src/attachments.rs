use crate::constants::ATTACHMENTS_DIR;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum OperationType {
    Copy,
    Move,
    Link,
}

impl OperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Copy => "COPY",
            Self::Move => "MOVE",
            Self::Link => "LINK",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub original_name: String,
    pub operation_type: OperationType,
    pub local_path: String,
    pub mime_type: String,
}

#[tauri::command]
pub async fn attach_file_copy(
    note_id: String,
    source_path: String,
    original_name: String,
    mime_type: String,
    state: State<'_, AppState>,
) -> Result<Attachment, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let workspace = {
        let ws_guard = state.workspace_path.lock().unwrap();
        ws_guard
            .as_ref()
            .ok_or("Workspace path not loaded")?
            .clone()
    };

    let id = Uuid::new_v4().to_string();

    // Get the file extension from the original filename
    let ext = Path::new(&original_name)
        .extension()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("");

    // Generate a secure UUID.extension filename
    let new_file_name = if ext.is_empty() {
        id.clone()
    } else {
        format!("{}.{}", id, ext)
    };

    let target_path = Path::new(&workspace)
        .join(ATTACHMENTS_DIR)
        .join(&new_file_name);

    // Copy the file to the hidden folder
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    let op_type = OperationType::Copy;
    let op_type_str = op_type.as_str();

    // Save a record to the database
    let result = sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, ?, ?, ?)",
        id, note_id, original_name, op_type_str, new_file_name, mime_type
    )
    .execute(pool)
    .await;

    // Check whether the database accepted the data
    match result {
        Ok(_) => Ok(Attachment {
            id,
            note_id,
            original_name,
            operation_type: op_type,
            local_path: new_file_name,
            mime_type,
        }),
        Err(e) => {
            let _ = fs::remove_file(target_path);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn attach_file_move(
    note_id: String,
    source_path: String,
    original_name: String,
    mime_type: String,
    state: State<'_, AppState>,
) -> Result<Attachment, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let workspace = {
        let ws_guard = state.workspace_path.lock().unwrap();
        ws_guard
            .as_ref()
            .ok_or("Workspace path not loaded")?
            .clone()
    };

    let id = Uuid::new_v4().to_string();

    let ext = Path::new(&original_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let new_file_name = if ext.is_empty() {
        id.clone()
    } else {
        format!("{}.{}", id, ext)
    };

    let target_path = Path::new(&workspace)
        .join(ATTACHMENTS_DIR)
        .join(&new_file_name);

    // Use a fallback on Windows when moving a file between C: and D: partitions
    if fs::rename(&source_path, &target_path).is_err() {
        fs::copy(&source_path, &target_path).map_err(|e| format!("Move failed: {}", e))?;

        let _ = fs::remove_file(&source_path);
    }

    let op_type = OperationType::Move;
    let op_type_str = op_type.as_str();

    sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, ?, ?, ?)",
        id, note_id, original_name, op_type_str, new_file_name, mime_type
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Attachment {
        id,
        note_id,
        original_name,
        operation_type: op_type,
        local_path: new_file_name,
        mime_type,
    })
}

#[tauri::command]
pub async fn attach_file_link(
    note_id: String,
    source_path: String,
    original_name: String,
    mime_type: String,
    state: State<'_, AppState>,
) -> Result<Attachment, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let op_type = OperationType::Link;
    let op_type_str = op_type.as_str();
    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, ?, ?, ?)",
        id, note_id, original_name, op_type_str, source_path, mime_type
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Attachment {
        id,
        note_id,
        original_name,
        operation_type: op_type,
        local_path: source_path,
        mime_type,
    })
}

#[tauri::command]
pub async fn attach_blob(
    note_id: String,
    bytes: Vec<u8>,
    original_name: String,
    mime_type: String,
    state: State<'_, AppState>,
) -> Result<Attachment, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let workspace = {
        let ws_guard = state.workspace_path.lock().unwrap();
        ws_guard
            .as_ref()
            .ok_or("Workspace path not loaded")?
            .clone()
    };

    let id = Uuid::new_v4().to_string();

    let extension = std::path::Path::new(&original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");

    let new_file_name = format!("{}.{}", id, extension);

    let target_path = Path::new(&workspace)
        .join(ATTACHMENTS_DIR)
        .join(&new_file_name);

    fs::write(&target_path, bytes).map_err(|e| format!("Failed to write blob: {}", e))?;

    let op_type = OperationType::Copy;
    let op_type_str = op_type.as_str();

    sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, ?, ?, ?)",
        id, note_id, original_name, op_type_str, new_file_name, mime_type
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Attachment {
        id,
        note_id,
        original_name,
        operation_type: op_type,
        local_path: new_file_name,
        mime_type,
    })
}
