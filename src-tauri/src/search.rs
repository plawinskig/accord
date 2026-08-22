use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub note_id: String,
    pub folder_id: String,
    pub folder_name: String,
    pub snippet: String,
    pub created_at: String,
}

#[tauri::command]
pub async fn search_notes(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    // to avoid FTS5 syntax errors,
    // add an asterisk at the end, which allows for partial-word searches
    // also replace double quotation marks so as not to break the query
    let safe_query = format!("\"{}\"*", query.replace("\"", "\"\""));

    let results = sqlx::query_as!(
        SearchResult,
        r#"
        WITH RECURSIVE deleted_folders AS (
            SELECT id FROM folders WHERE is_deleted = 1
            UNION ALL
            SELECT f.id FROM folders f
            INNER JOIN deleted_folders df ON f.parent_id = df.id
        )
        SELECT 
            n.id as "note_id!", 
            n.folder_id as "folder_id!", 
            f.name as "folder_name!",
            snippet(notes_fts, 0, '<mark class="bg-indigo-500/80 text-white rounded px-1">', '</mark>', '...', 15) as "snippet!: String",
            DATETIME(n.created_at, 'localtime') as "created_at!: String"
        FROM notes_fts fts
        JOIN notes n ON fts.note_id = n.id
        JOIN folders f ON n.folder_id = f.id
        WHERE notes_fts MATCH ? 
          AND n.is_deleted = 0
          AND n.folder_id NOT IN (SELECT id FROM deleted_folders)
        ORDER BY rank
        LIMIT 20
        "#,
        safe_query
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(results)
}
