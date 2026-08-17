use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub original_name: String,
    pub operation_type: String,
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
    
    // 1. Wyciągamy rozszerzenie z oryginalnej nazwy (np. "png", "pdf")
    let ext = Path::new(&original_name)
        .extension()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("");
        
    // 2. Generujemy bezpieczną nazwę piku (UUID.rozszerzenie)
    let new_file_name = if ext.is_empty() {
        id.clone()
    } else {
        format!("{}.{}", id, ext)
    };

    let target_path = Path::new(workspace).join("attachments").join(&new_file_name);

    // 3. Kopiujemy plik fizycznie do naszego ukrytego folderu
    fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    // 4. TRANSAKCYJNOŚĆ: Próbujemy zapisać wpis w bazie danych
    let result = sqlx::query!(
        "INSERT INTO attachments (id, note_id, original_name, operation_type, local_path, mime_type) VALUES (?, ?, ?, 'COPY', ?, ?)",
        id, note_id, original_name, new_file_name, mime_type
    )
    .execute(pool)
    .await;

    // 5. Sprawdzamy, czy baza przyjęła dane
    match result {
        Ok(_) => {
            Ok(Attachment {
                id,
                note_id,
                original_name,
                operation_type: "COPY".to_string(),
                local_path: new_file_name,
                mime_type,
            })
        }
        Err(e) => {
            // ROLLBACK: Baza rzuciła błąd (np. brak notatki o takim ID). 
            // Cicho usuwamy skopiowany plik z dysku, żeby nie śmiecić!
            let _ = fs::remove_file(target_path); 
            Err(e.to_string())
        }
    }
}