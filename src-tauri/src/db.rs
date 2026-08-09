use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use tauri::State;
use crate::AppState;

// internal function connecting to the database in workspace
pub async fn init_db(workspace_path: &str) -> Result<SqlitePool, String> {
    let db_path = Path::new(workspace_path).join("accord.sqlite");
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());

    // set options: create a file if it does not exist and use WAL mode for security
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| e.to_string())?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(|e| e.to_string())?;

    // searches the "./migrations" folder and applies undone migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;

    Ok(pool)
}

// command called right after the path is loaded
#[tauri::command]
pub async fn connect_to_db(workspace_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let pool = init_db(&workspace_path).await?;
    
    // save the connection pool in the application's memory (state) so that other functions can access it
    *state.db.lock().await = Some(pool);
    
    Ok(())
}