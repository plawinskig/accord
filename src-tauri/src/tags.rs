use crate::AppState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

// Matches a `#` followed by one or more Unicode letters/digits/underscores,
// so e.g. #ważne (Polish diacritics) and #project_2 both match. \p{L}/\p{N}
// require the `unicode` feature of the `regex` crate, which is on by default.
fn tag_token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"#([\p{L}\p{N}_]+)").unwrap())
}

// Full-match validator used for tags coming from the manual tag-picker menu,
// which isn't constrained by the "#" trigger and could otherwise contain
// stray characters (spaces, commas) that would break the GROUP_CONCAT
// aggregation used in notes::get_notes.
fn tag_name_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[\p{L}\p{N}_]+$").unwrap())
}

/// Extracts unique `#tag` tokens from note content, normalized to lowercase.
/// Rust's `to_lowercase` is Unicode-aware (unlike SQLite's built-in NOCASE
/// collation, which is ASCII-only), so this is the single source of case
/// normalization - the `tags.name` column always stores the lowercase form.
pub fn parse_tags_from_content(content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    tag_token_regex()
        .captures_iter(content)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_lowercase()))
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

fn normalize_tag_name(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return Err("Tag name cannot be empty".into());
    }
    if !tag_name_regex().is_match(&normalized) {
        return Err("Tag name can only contain letters, digits, and underscores".into());
    }
    Ok(normalized)
}

async fn find_or_create_tag(pool: &sqlx::SqlitePool, name: &str) -> Result<String, sqlx::Error> {
    if let Some(row) = sqlx::query!(r#"SELECT id as "id!" FROM tags WHERE name = ?"#, name)
        .fetch_optional(pool)
        .await?
    {
        return Ok(row.id);
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query!("INSERT INTO tags (id, name) VALUES (?, ?)", id, name)
        .execute(pool)
        .await?;
    Ok(id)
}

/// Ensures every given tag name exists in `tags` and is linked to `note_id`
/// in `note_tags`. Shared by the `#tag` auto-detection (create_note /
/// update_note) and by the manual attach_tag command - both paths converge
/// on the same note_tags table, so it doesn't matter which UI the user used.
pub async fn ensure_tags_for_note(
    pool: &sqlx::SqlitePool,
    note_id: &str,
    tag_names: &[String],
) -> Result<(), sqlx::Error> {
    for name in tag_names {
        let tag_id = find_or_create_tag(pool, name).await?;
        sqlx::query!(
            "INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?, ?)",
            note_id,
            tag_id
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Returns every tag in the workspace, for the global list in the right
/// sidebar (Krok 3).
#[tauri::command]
pub async fn get_all_tags(state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query_as!(
        Tag,
        r#"SELECT id as "id!", name as "name!" FROM tags ORDER BY name ASC"#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Attaches a tag (by name) to a note from the manual tag-picker menu.
/// Creates the tag if it doesn't already exist - same underlying path as
/// `#tag` auto-detection.
#[tauri::command]
pub async fn attach_tag(
    note_id: String,
    tag_name: String,
    state: State<'_, AppState>,
) -> Result<Tag, String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let normalized = normalize_tag_name(&tag_name)?;
    let tag_id = find_or_create_tag(pool, &normalized)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query!(
        "INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?, ?)",
        note_id,
        tag_id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Tag {
        id: tag_id,
        name: normalized,
    })
}

/// Unlinks a tag from a single note. Does NOT delete the tag globally - use
/// `delete_tag` for that.
#[tauri::command]
pub async fn detach_tag(
    note_id: String,
    tag_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query!(
        "DELETE FROM note_tags WHERE note_id = ? AND tag_id = ?",
        note_id,
        tag_id
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Renames a tag everywhere it's used (note_tags links are untouched, since
/// they reference the tag by id, not by name).
#[tauri::command]
pub async fn rename_tag(
    id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    let normalized = normalize_tag_name(&new_name)?;

    sqlx::query!("UPDATE tags SET name = ? WHERE id = ?", normalized, id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Deletes a tag globally. Relies on `ON DELETE CASCADE` on note_tags.tag_id
/// (requires `PRAGMA foreign_keys = ON`, enabled in db.rs) to clean up every
/// note_tags row referencing it - no manual cleanup needed here.
#[tauri::command]
pub async fn delete_tag(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state.db.lock().await;
    let pool = db_guard.as_ref().ok_or("Database not connected")?;

    sqlx::query!("DELETE FROM tags WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_tags() {
        assert_eq!(
            parse_tags_from_content("Oto projekt #ważne i #todo"),
            vec!["ważne".to_string(), "todo".to_string()]
        );
    }

    #[test]
    fn deduplicates_and_lowercases() {
        assert_eq!(
            parse_tags_from_content("#Ważne coś tam #WAŻNE znowu"),
            vec!["ważne".to_string()]
        );
    }

    #[test]
    fn ignores_bare_hash() {
        assert_eq!(
            parse_tags_from_content("cena to # 5 zł"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn validates_tag_names() {
        assert!(normalize_tag_name("ważne").is_ok());
        assert!(normalize_tag_name("  Project_2 ").is_ok());
        assert!(normalize_tag_name("").is_err());
        assert!(normalize_tag_name("bad tag").is_err());
        assert!(normalize_tag_name("bad,tag").is_err());
    }
}
