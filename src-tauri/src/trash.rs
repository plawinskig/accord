use crate::constants::ATTACHMENTS_DIR;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrashedItem {
    pub id: String,
    pub item_type: String,
    pub name: String,
}

#[tauri::command]
pub async fn get_trash(state: State<'_, AppState>) -> Result<Vec<TrashedItem>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let mut items = Vec::new();

    // 1. Pobieramy foldery
    let folders =
        sqlx::query!(r#"SELECT id as "id!", name as "name!" FROM folders WHERE is_deleted = 1"#)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

    for f in folders {
        items.push(TrashedItem {
            id: f.id,
            item_type: "folder".to_string(),
            name: format!("# {}", f.name),
        });
    }

    // 2. Pobieramy notatki
    let notes = sqlx::query!(
        r#"SELECT id as "id!", content as "content!" FROM notes WHERE is_deleted = 1"#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for n in notes {
        // Skracamy treść dla podglądu w koszu
        let snippet = if n.content.len() > 30 {
            format!("{}...", &n.content[..30])
        } else {
            n.content
        };
        items.push(TrashedItem {
            id: n.id,
            item_type: "note".to_string(),
            name: snippet,
        });
    }

    Ok(items)
}

#[tauri::command]
pub async fn restore_item(
    id: String,
    item_type: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    if item_type == "folder" {
        sqlx::query!("UPDATE folders SET is_deleted = 0 WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        sqlx::query!("UPDATE notes SET is_deleted = 0 WHERE id = ?", id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let workspace = {
        let ws_guard = state.workspace_path.lock().unwrap();
        ws_guard
            .as_ref()
            .ok_or("Workspace path not loaded")?
            .clone()
    };

    // 1. Wyszukujemy wszystkie załączniki należące do usuniętych notatek ORAZ notatek wewnątrz usuniętych folderów (Rekurencja CTE)
    let attachments = sqlx::query!(
        r#"
        SELECT a.local_path as "local_path!", a.operation_type as "operation_type!: crate::attachments::OperationType"
        FROM attachments a
        JOIN notes n ON a.note_id = n.id
        WHERE n.is_deleted = 1 OR n.folder_id IN (
            WITH RECURSIVE deleted_folders AS (
                SELECT id FROM folders WHERE is_deleted = 1
                UNION ALL SELECT f.id FROM folders f INNER JOIN deleted_folders df ON f.parent_id = df.id
            ) SELECT id FROM deleted_folders
        )
        "#
    ).fetch_all(pool).await.map_err(|e| e.to_string())?;

    // 2. Usuwamy fizyczne pliki z dysku (jeśli były kopiowane lub przenoszone - ignorujemy linki)
    for att in attachments {
        if att.operation_type != crate::attachments::OperationType::Link {
            let _ = fs::remove_file(
                Path::new(&workspace)
                    .join(ATTACHMENTS_DIR)
                    .join(att.local_path),
            );
        }
    }

    // 3. Usuwamy notatki (Klucze ON DELETE CASCADE automatycznie usuną wpisy z note_tags oraz attachments!)
    sqlx::query!(
        r#"
        DELETE FROM notes WHERE is_deleted = 1 OR folder_id IN (
            WITH RECURSIVE deleted_folders AS (
                SELECT id FROM folders WHERE is_deleted = 1
                UNION ALL SELECT f.id FROM folders f INNER JOIN deleted_folders df ON f.parent_id = df.id
            ) SELECT id FROM deleted_folders
        )
        "#
    ).execute(pool).await.map_err(|e| e.to_string())?;

    // 4. Usuwamy foldery
    sqlx::query!("DELETE FROM folders WHERE is_deleted = 1")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
