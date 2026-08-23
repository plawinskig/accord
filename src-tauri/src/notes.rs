use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

// Download all notes from a specific folder, along with their attachments,
// in a single query (LEFT JOIN) instead of one attachments query per note.
#[tauri::command]
pub async fn get_notes(folder_id: String, state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    // Every row is a note, optionally joined with one of its attachments.
    // A note with no attachments produces exactly one row with the `att_*`
    // columns as NULL; a note with N attachments produces N rows. The
    // `?` suffix tells the sqlx macro these columns are nullable (LEFT JOIN).
    let rows = sqlx::query!(
        r#"
        SELECT
            n.id as "note_id!",
            n.folder_id as "folder_id!",
            n.content as "content!",
            DATETIME(n.created_at, 'localtime') as "created_at!: String",
            DATETIME(n.updated_at, 'localtime') as "updated_at!: String",
            a.id as "att_id?",
            a.note_id as "att_note_id?",
            a.original_name as "att_original_name?",
            a.operation_type as "att_operation_type?: crate::attachments::OperationType",
            a.local_path as "att_local_path?",
            a.mime_type as "att_mime_type?"
        FROM notes n
        LEFT JOIN attachments a ON a.note_id = n.id
        WHERE n.folder_id = ? AND n.is_deleted = 0
        ORDER BY n.created_at ASC
        "#,
        folder_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Group rows back into notes while preserving the ORDER BY from the query.
    let mut notes: Vec<Note> = Vec::new();
    let mut index_by_note_id: HashMap<String, usize> = HashMap::new();

    for row in rows {
        let note_idx = *index_by_note_id
            .entry(row.note_id.clone())
            .or_insert_with(|| {
                notes.push(Note {
                    id: row.note_id.clone(),
                    folder_id: row.folder_id.clone(),
                    content: row.content.clone(),
                    created_at: row.created_at.clone(),
                    updated_at: row.updated_at.clone(),
                    attachments: Some(Vec::new()),
                });
                notes.len() - 1
            });

        if let (
            Some(att_id),
            Some(att_note_id),
            Some(att_original_name),
            Some(att_operation_type),
            Some(att_local_path),
            Some(att_mime_type),
        ) = (
            row.att_id,
            row.att_note_id,
            row.att_original_name,
            row.att_operation_type,
            row.att_local_path,
            row.att_mime_type,
        ) {
            notes[note_idx]
                .attachments
                .as_mut()
                .unwrap()
                .push(crate::attachments::Attachment {
                    id: att_id,
                    note_id: att_note_id,
                    original_name: att_original_name,
                    operation_type: att_operation_type,
                    local_path: att_local_path,
                    mime_type: att_mime_type,
                });
        }
    }

    Ok(notes)
}

// Add a new note
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

// Soft-delete the note to the trash
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
