use crate::constants::DB_FILE_NAME;
use crate::AppState;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::path::Path;
use std::str::FromStr;
use tauri::State;

// Connect to the workspace database internally
pub async fn init_db(workspace_path: &str) -> Result<SqlitePool, String> {
    let db_path = Path::new(workspace_path).join(DB_FILE_NAME);
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // Set options to create a missing file and use WAL mode for security
    // Enable foreign key enforcement. SQLite has this OFF by default per
    // connection - without it, every `ON DELETE CASCADE` in the schema
    // (attachments -> notes, and now note_tags -> notes/tags) is silently
    // ignored and orphaned rows are left behind on hard delete.
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| e.to_string())?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(|e| e.to_string())?;

    // Search the "./migrations" folder and apply pending migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;

    Ok(pool)
}

// Call this command immediately after loading the path
#[tauri::command]
pub async fn connect_to_db(
    workspace_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pool = init_db(&workspace_path).await?;

    // Save the connection pool in application state so other functions can access it
    *state.db.lock().await = Some(pool);
    // Save the path so the attachments module can access it
    *state.workspace_path.lock().unwrap() = Some(workspace_path);

    Ok(())
}
