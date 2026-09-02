use crate::attachments::OperationType;
use crate::AppState;
use std::fs;
use tauri::State;

#[tauri::command]
pub async fn export_folder(
    folder_id: String,
    dest_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let folder = sqlx::query!(
        r#"SELECT name as "name!" FROM folders WHERE id = ?"#,
        folder_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let notes = sqlx::query!(
        r#"
        SELECT id as "id!", content as "content!", DATETIME(created_at, 'localtime') as "created_at!: String"
        FROM notes WHERE folder_id = ? AND is_deleted = 0 ORDER BY created_at ASC
        "#,
        folder_id
    ).fetch_all(pool).await.map_err(|e| e.to_string())?;

    let mut md = format!("# {}\n\n", folder.name);

    for note in notes {
        md.push_str(&format!("**{}**\n{}\n", note.created_at, note.content));

        let atts = sqlx::query!(
            r#"SELECT original_name as "original_name!", operation_type as "operation_type!: OperationType", local_path as "local_path!" FROM attachments WHERE note_id = ?"#,
            note.id
        ).fetch_all(pool).await.map_err(|e| e.to_string())?;

        if !atts.is_empty() {
            md.push_str("\n*Attachments:*\n");
            for att in atts {
                let link_path = if att.operation_type == OperationType::Link {
                    att.local_path
                } else {
                    format!("./attachments/{}", att.local_path)
                };
                md.push_str(&format!("- [{}]({})\n", att.original_name, link_path));
            }
        }
        md.push_str("\n---\n\n");
    }

    fs::write(&dest_path, md).map_err(|e| e.to_string())?;

    Ok(())
}
