use crate::state::AppState;
use base64::engine::{Engine as _, general_purpose};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
pub struct Attachment {
    pub id: u32,
    pub entity_type: String,
    pub entity_id: u32,
    pub filename: String,
    pub original_name: String,
    pub mime_type: String,
    pub file_size: u64,
    pub sha256: String,
    pub sort_order: i32,
    pub created_at: i64,
}

#[derive(Deserialize)]
pub struct AttachmentInput {
    pub entity_type: String,
    pub entity_id: u32,
    pub filename: String,
    pub original_name: String,
    pub mime_type: String,
    pub file_data_base64: String,
}

fn attachments_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("attachments")
}

fn entity_dir(data_dir: &Path, entity_type: &str, entity_id: u32) -> PathBuf {
    attachments_dir(data_dir)
        .join(entity_type)
        .join(entity_id.to_string())
}

fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe.chars().take(255).collect()
}

fn ensure_unique_filename(dir: &Path, filename: &str) -> String {
    let base = dir.join(filename);
    if !base.exists() {
        return filename.to_string();
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    for i in 2..=1000 {
        let candidate = if ext.is_empty() {
            format!("{}_{}", stem, i)
        } else {
            format!("{}_{}.{}", stem, i, ext)
        };
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }
    filename.to_string()
}

fn validate_attachment(input: &AttachmentInput) -> Result<(), String> {
    if !matches!(
        input.entity_type.as_str(),
        "finding" | "engagement" | "document"
    ) {
        return Err("Invalid entity_type. Must be finding, engagement, or document.".to_string());
    }
    let max_size = 50 * 1024 * 1024; // 50MB
    let decoded_len = input.file_data_base64.len() * 3 / 4;
    if decoded_len > max_size {
        return Err("File exceeds 50MB limit.".to_string());
    }
    Ok(())
}

fn do_upload_attachment(
    conn: &Connection,
    data_dir: &Path,
    input: &AttachmentInput,
) -> Result<Attachment, String> {
    validate_attachment(input)?;

    let decoded = general_purpose::STANDARD
        .decode(&input.file_data_base64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    if decoded.is_empty() {
        return Err("Empty file data.".to_string());
    }

    let hash = hex::encode(Sha256::digest(&decoded));
    let safe_name = sanitize_filename(&input.filename);
    let entity_path = entity_dir(data_dir, &input.entity_type, input.entity_id);
    std::fs::create_dir_all(&entity_path)
        .map_err(|e| format!("Failed to create attachment directory: {}", e))?;

    let unique_name = ensure_unique_filename(&entity_path, &safe_name);
    let file_path = entity_path.join(&unique_name);
    std::fs::write(&file_path, &decoded)
        .map_err(|e| format!("Failed to write attachment file: {}", e))?;

    conn.execute(
        "INSERT INTO attachments (entity_type, entity_id, filename, original_name, mime_type, file_size, sha256, sort_order, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%s', 'now'))",
        params![
            input.entity_type,
            input.entity_id,
            unique_name,
            input.original_name,
            input.mime_type,
            decoded.len() as i64,
            hash,
            0,
        ],
    )
    .map_err(|e| format!("Failed to insert attachment record: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    do_get_attachment(conn, id)
}

fn do_get_attachment(conn: &Connection, id: u32) -> Result<Attachment, String> {
    let item: Option<Attachment> = conn
        .query_row(
            "SELECT id, entity_type, entity_id, filename, original_name, mime_type, file_size, sha256, sort_order, created_at
             FROM attachments WHERE id = ? AND is_active = 1",
            params![id],
            |row| {
                Ok(Attachment {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    entity_id: row.get(2)?,
                    filename: row.get(3)?,
                    original_name: row.get(4)?,
                    mime_type: row.get(5)?,
                    file_size: row.get::<_, i64>(6)? as u64,
                    sha256: row.get(7)?,
                    sort_order: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;
    item.ok_or_else(|| "Attachment not found.".to_string())
}

fn do_list_attachments(
    conn: &Connection,
    entity_type: &str,
    entity_id: u32,
) -> Result<Vec<Attachment>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, entity_type, entity_id, filename, original_name, mime_type, file_size, sha256, sort_order, created_at
             FROM attachments WHERE entity_type = ? AND entity_id = ? AND is_active = 1 ORDER BY sort_order, created_at DESC",
        )
        .map_err(|e| format!("Database error: {}", e))?;
    let rows = stmt
        .query_map(params![entity_type, entity_id], |row| {
            Ok(Attachment {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                entity_id: row.get(2)?,
                filename: row.get(3)?,
                original_name: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get::<_, i64>(6)? as u64,
                sha256: row.get(7)?,
                sort_order: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Database error: {}", e))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {}", e))?);
    }
    Ok(items)
}

fn do_delete_attachment(conn: &Connection, data_dir: &Path, id: u32) -> Result<(), String> {
    let att = do_get_attachment(conn, id)?;
    let file_path = entity_dir(data_dir, &att.entity_type, att.entity_id).join(&att.filename);
    if file_path.exists() {
        std::fs::remove_file(&file_path).map_err(|e| format!("Failed to remove file: {}", e))?;
    }
    conn.execute("DELETE FROM attachments WHERE id = ?", params![id])
        .map_err(|e| format!("Failed to delete attachment: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["attachments", "DELETE", id, "", "", format!("entity_type={} entity_id={}", att.entity_type, att.entity_id)],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

fn do_rename_attachment(conn: &Connection, id: u32, new_name: &str) -> Result<(), String> {
    let safe = sanitize_filename(new_name);
    conn.execute(
        "UPDATE attachments SET original_name = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![safe, id],
    )
    .map_err(|e| format!("Failed to rename attachment: {}", e))?;
    Ok(())
}

fn do_reorder_attachments(
    conn: &Connection,
    entity_type: &str,
    entity_id: u32,
    ordered_ids: &[u32],
) -> Result<(), String> {
    for (i, id) in ordered_ids.iter().enumerate() {
        conn.execute(
            "UPDATE attachments SET sort_order = ? WHERE id = ? AND entity_type = ? AND entity_id = ?",
            params![i as i32, id, entity_type, entity_id],
        )
        .map_err(|e| format!("Failed to reorder: {}", e))?;
    }
    Ok(())
}

fn do_read_attachment_file(
    data_dir: &Path,
    entity_type: &str,
    entity_id: u32,
    filename: &str,
) -> Result<Vec<u8>, String> {
    let path = entity_dir(data_dir, entity_type, entity_id).join(filename);
    std::fs::read(&path).map_err(|e| format!("Failed to read attachment file: {}", e))
}

fn get_data_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("App data dir: {}", e))?;
    Ok(data_dir)
}

// ─────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn upload_attachment(
    state: State<AppState>,
    app_handle: AppHandle,
    input: AttachmentInput,
) -> Result<Attachment, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    let data_dir = get_data_dir(&app_handle)?;
    do_upload_attachment(conn, &data_dir, &input)
}

#[tauri::command]
pub fn list_attachments(
    state: State<AppState>,
    entity_type: String,
    entity_id: u32,
) -> Result<Vec<Attachment>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    do_list_attachments(conn, &entity_type, entity_id)
}

#[tauri::command]
pub fn delete_attachment(
    state: State<AppState>,
    app_handle: AppHandle,
    id: u32,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    let data_dir = get_data_dir(&app_handle)?;
    do_delete_attachment(conn, &data_dir, id)
}

#[tauri::command]
pub fn rename_attachment(state: State<AppState>, id: u32, new_name: String) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_rename_attachment(conn, id, &new_name)
}

#[tauri::command]
pub fn reorder_attachments(
    state: State<AppState>,
    entity_type: String,
    entity_id: u32,
    ordered_ids: Vec<u32>,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_reorder_attachments(conn, &entity_type, entity_id, &ordered_ids)
}

#[tauri::command]
pub fn read_attachment_file(
    app_handle: AppHandle,
    entity_type: String,
    entity_id: u32,
    filename: String,
) -> Result<String, String> {
    let data_dir = get_data_dir(&app_handle)?;
    let bytes = do_read_attachment_file(&data_dir, &entity_type, entity_id, &filename)?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub fn get_total_attachment_storage(state: State<AppState>) -> Result<u64, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM attachments WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;
    Ok(total as u64)
}

#[tauri::command]
pub fn get_attachment_thumbnail(
    app_handle: AppHandle,
    entity_type: String,
    entity_id: u32,
    filename: String,
) -> Result<String, String> {
    let data_dir = get_data_dir(&app_handle)?;
    let file_path = entity_dir(&data_dir, &entity_type, entity_id).join(&filename);
    let bytes = std::fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let thumb = generate_thumbnail(&bytes, 120)
        .map_err(|e| format!("Thumbnail generation failed: {}", e))?;
    Ok(general_purpose::STANDARD.encode(&thumb))
}

fn generate_thumbnail(data: &[u8], max_dim: u32) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Invalid image: {}", e))?;
    let w = img.width();
    let h = img.height();
    let ratio = w as f32 / h as f32;
    let (nw, nh) = if w > h {
        (max_dim, (max_dim as f32 / ratio) as u32)
    } else {
        ((max_dim as f32 * ratio) as u32, max_dim)
    };
    let thumb = img.resize(nw, nh, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    thumb
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("Write failed: {}", e))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let key = [8u8; 32];
        let conn = db::open_vault(tmp.path(), &key).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_attachment_crud() {
        let conn = test_conn();
        let tmp = tempfile::tempdir().unwrap();
        let input = AttachmentInput {
            entity_type: "finding".to_string(),
            entity_id: 1,
            filename: "test.png".to_string(),
            original_name: "test.png".to_string(),
            mime_type: "image/png".to_string(),
            file_data_base64: base64::encode(b"hello"),
        };
        let att = do_upload_attachment(&conn, tmp.path(), &input).unwrap();
        assert_eq!(att.entity_type, "finding");
        assert_eq!(att.file_size, 5);

        let list = do_list_attachments(&conn, "finding", 1).unwrap();
        assert_eq!(list.len(), 1);

        do_rename_attachment(&conn, att.id, "renamed.png").unwrap();
        do_delete_attachment(&conn, tmp.path(), att.id).unwrap();
        let list = do_list_attachments(&conn, "finding", 1).unwrap();
        assert!(list.is_empty());
    }
}
