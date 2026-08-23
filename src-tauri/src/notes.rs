use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub folder_id: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub attachments: Option<Vec<crate::attachments::Attachment>>,
}

// download all notes from a specific folder
#[tauri::command]
pub async fn get_notes(folder_id: String, state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let notes_records = sqlx::query!(
        r#"
        SELECT 
            id as "id!", 
            folder_id as "folder_id!", 
            content as "content!", 
            DATETIME(created_at, 'localtime') as "created_at!: String", 
            DATETIME(updated_at, 'localtime') as "updated_at!: String"
        FROM notes 
        WHERE folder_id = ? AND is_deleted = 0 
        ORDER BY created_at ASC
        "#,
        folder_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut notes = Vec::new();

    // SLOW N+1
    for record in notes_records {
        let atts = sqlx::query_as!(
            crate::attachments::Attachment,
            r#"
            SELECT 
                id as "id!", 
                note_id as "note_id!", 
                original_name as "original_name!", 
                operation_type as "operation_type!: crate::attachments::OperationType", 
                local_path as "local_path!", 
                mime_type as "mime_type!" 
            FROM attachments 
            WHERE note_id = ?
            "#,
            record.id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        notes.push(Note {
            id: record.id,
            folder_id: record.folder_id,
            content: record.content,
            created_at: record.created_at,
            updated_at: record.updated_at,
            attachments: Some(atts),
        });
    }

    Ok(notes)
}

// Add new note
#[tauri::command]
pub async fn create_note(
    folder_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Note, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        "INSERT INTO notes (id, folder_id, content, is_deleted) VALUES (?, ?, ?, 0)",
        id,
        folder_id,
        content
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let record = sqlx::query!(
        r#"
        SELECT 
            id as "id!", 
            folder_id as "folder_id!", 
            content as "content!", 
            DATETIME(created_at, 'localtime') as "created_at!: String", 
            DATETIME(updated_at, 'localtime') as "updated_at!: String" 
        FROM notes 
        WHERE id = ?
        "#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Note {
        id: record.id,
        folder_id: record.folder_id,
        content: record.content,
        created_at: record.created_at,
        updated_at: record.updated_at,
        attachments: Some(vec![]),
    })
}

// Edit the note
#[tauri::command]
pub async fn update_note(
    id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query!(
        "UPDATE notes SET content = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        content,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

// Soft delete to trash
#[tauri::command]
pub async fn soft_delete_note(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query!("UPDATE notes SET is_deleted = 1 WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
