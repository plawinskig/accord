use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;
use crate::AppState;
use crate::constants;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
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
    
    let ws_guard = state.workspace_path.lock().await;
    let workspace = ws_guard.as_ref().ok_or("Workspace path not loaded")?;

    let id = Uuid::new_v4().to_string();
    
    // get file extension from the original filename
    let ext = Path::new(&original_name)
        .extension()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("");
        
    // generate a secure file name UUID.extension
    let new_file_name = if ext.is_empty() {
        id.clone()
    } else {
        format!("{}.{}", id, ext)
    };

    let target_path = Path::new(workspace).join(constants::ATTACHMENTS_DIR).join(&new_file_name);

    // copy the file to the hidden folder
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    let op_type = OperationType::Copy;
    let op_type_str = op_type.as_str();

    // attempt to save a record to the database
    let result = sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, ?, ?, ?)",
        id, note_id, original_name, op_type_str, new_file_name, mime_type
    )
    .execute(pool)
    .await;

    // check to see if the database has accepted the data
    match result {
        Ok(_) => {
            Ok(Attachment {
                id,
                note_id,
                original_name,
                operation_type: op_type,
                local_path: new_file_name,
                mime_type,
            })
        }
        Err(e) => {
            let _ = fs::remove_file(target_path); 
            Err(e.to_string())
        }
    }
}
