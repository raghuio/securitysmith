use crate::error::AppError;
use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ExportSummary {
    pub clients: u32,
    pub engagements: u32,
    pub findings: u32,
    pub credentials: u32,
    pub documents: u32,
    pub invoices: u32,
    pub templates: u32,
    pub reports: u32,
    pub file_path: String,
}

#[derive(Debug, Serialize)]
pub struct ExportTreeClient {
    pub id: u32,
    pub name: String,
    pub engagements: Vec<ExportTreeEngagement>,
    pub documents: Vec<ExportTreeDocument>,
    pub invoices: Vec<ExportTreeInvoice>,
}

#[derive(Debug, Serialize)]
pub struct ExportTreeEngagement {
    pub id: u32,
    pub name: String,
    pub finding_count: u32,
    pub credential_count: u32,
    pub document_count: u32,
    pub finding_ids: Vec<u32>,
    pub credential_ids: Vec<u32>,
    pub document_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct ExportTreeDocument {
    pub id: u32,
    pub name: String,
    pub document_type: String,
}

#[derive(Debug, Serialize)]
pub struct ExportTreeInvoice {
    pub id: u32,
    pub invoice_number: String,
}

#[derive(Debug, Serialize)]
pub struct ExportTreeTemplate {
    pub id: u32,
    pub name: String,
    pub category: String,
}

#[derive(Debug, Serialize)]
pub struct ExportTree {
    pub clients: Vec<ExportTreeClient>,
    pub templates: Vec<ExportTreeTemplate>,
}

#[derive(Deserialize)]
pub struct ExportSelection {
    pub client_ids: Vec<u32>,
    pub engagement_ids: Vec<u32>,
    pub finding_ids: Vec<u32>,
    pub credential_ids: Vec<u32>,
    pub document_ids: Vec<u32>,
    pub invoice_ids: Vec<u32>,
    pub template_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub file_path: String,
    pub entity_counts: HashMap<String, u32>,
}

#[derive(Debug, Serialize)]
pub struct ImportPreview {
    pub compatible: bool,
    pub conflicts: Vec<ImportConflict>,
}

#[derive(Debug, Serialize)]
pub struct ImportConflict {
    pub entity_type: String,
    pub import_name: String,
}

#[derive(Deserialize)]
pub struct ConflictResolution {
    pub reference_key: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: HashMap<String, u32>,
    pub skipped: HashMap<String, u32>,
}

fn serialize_value(val: rusqlite::types::Value) -> serde_json::Value {
    match val {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(v) => serde_json::Value::Number(v.into()),
        rusqlite::types::Value::Real(v) => {
            serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into()))
        }
        rusqlite::types::Value::Text(v) => serde_json::Value::String(v),
        rusqlite::types::Value::Blob(v) => serde_json::Value::String(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &v,
        )),
    }
}

const EXPORT_TABLES: &[&str] = &[
    "clients", "engagements", "findings", "credentials", "documents",
    "invoices", "templates", "reports", "checklists", "compliance_frameworks",
    "compliance_controls", "news_articles", "feeds", "time_entries",
];

fn table_to_json(
    conn: &rusqlite::Connection,
    table: &str,
    ids: Option<&Vec<u32>>,
) -> Result<Vec<serde_json::Value>, String> {
    if !EXPORT_TABLES.contains(&table) {
        return Err(format!("Invalid export table: {}", table));
    }
    let sql = if let Some(ids) = ids {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        format!(
            "SELECT * FROM {} WHERE id IN ({})",
            table,
            placeholders.join(",")
        )
    } else {
        format!("SELECT * FROM {table}")
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("col").to_string())
        .collect();

    let rows: Vec<serde_json::Value> = if let Some(ids) = ids {
        let id_refs: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        stmt.query_map(rusqlite::params_from_iter(id_refs), |row| {
            let mut obj = serde_json::Map::<String, serde_json::Value>::new();
            for (i, name) in col_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                obj.insert(name.clone(), serialize_value(val));
            }
            Ok(serde_json::Value::Object(obj))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Warning: export row skipped: {e}");
                None
            }
        })
        .collect()
    } else {
        stmt.query_map([], |row| {
            let mut obj = serde_json::Map::<String, serde_json::Value>::new();
            for (i, name) in col_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                obj.insert(name.clone(), serialize_value(val));
            }
            Ok(serde_json::Value::Object(obj))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Warning: export row skipped: {e}");
                None
            }
        })
        .collect()
    };
    Ok(rows)
}

#[tauri::command]
pub fn get_export_tree(state: State<AppState>) -> Result<ExportTree, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let mut clients: Vec<ExportTreeClient> = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, name FROM clients WHERE is_active = 1 ORDER BY name")
        .map_err(AppError::from)?;
    let client_rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(AppError::from)?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    for (cid, cname) in client_rows {
        let mut engagements: Vec<ExportTreeEngagement> = Vec::new();
        let mut stmt2 = conn
            .prepare("SELECT id, name FROM engagements WHERE client_id = ? AND is_active = 1 ORDER BY name")
            .map_err(AppError::from)?;
        let eng_rows = stmt2
            .query_map(params![cid], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(AppError::from)?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        for (eid, ename) in eng_rows {
            let f_count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM findings WHERE engagement_id = ? AND is_active = 1",
                    [eid],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let c_count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM credentials WHERE engagement_id = ?",
                    [eid],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let d_count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM documents WHERE engagement_id = ? AND is_active = 1",
                    [eid],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let finding_ids: Vec<u32> = conn
                .prepare("SELECT id FROM findings WHERE engagement_id = ? AND is_active = 1")
                .unwrap()
                .query_map([eid], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            let credential_ids: Vec<u32> = conn
                .prepare("SELECT id FROM credentials WHERE engagement_id = ?")
                .unwrap()
                .query_map([eid], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            let document_ids: Vec<u32> = conn
                .prepare("SELECT id FROM documents WHERE engagement_id = ? AND is_active = 1")
                .unwrap()
                .query_map([eid], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            engagements.push(ExportTreeEngagement {
                id: eid,
                name: ename,
                finding_count: f_count,
                credential_count: c_count,
                document_count: d_count,
                finding_ids,
                credential_ids,
                document_ids,
            });
        }

        let mut documents: Vec<ExportTreeDocument> = Vec::new();
        let mut stmt3 = conn
            .prepare("SELECT id, name, document_type FROM documents WHERE client_id = ? AND is_active = 1 ORDER BY name")
            .map_err(AppError::from)?;
        let doc_rows = stmt3
            .query_map(params![cid], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(AppError::from)?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        for (did, dname, dtype) in doc_rows {
            documents.push(ExportTreeDocument {
                id: did,
                name: dname,
                document_type: dtype,
            });
        }

        let mut invoices: Vec<ExportTreeInvoice> = Vec::new();
        let mut stmt4 = conn
            .prepare("SELECT id, invoice_number FROM invoices WHERE client_id = ? AND is_active = 1 ORDER BY invoice_number")
            .map_err(AppError::from)?;
        let inv_rows = stmt4
            .query_map(params![cid], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(AppError::from)?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        for (iid, inum) in inv_rows {
            invoices.push(ExportTreeInvoice {
                id: iid,
                invoice_number: inum,
            });
        }

        clients.push(ExportTreeClient {
            id: cid,
            name: cname,
            engagements,
            documents,
            invoices,
        });
    }

    let mut templates: Vec<ExportTreeTemplate> = Vec::new();
    let mut stmt5 = conn
        .prepare("SELECT id, name, category FROM templates WHERE is_builtin = 0 AND is_active = 1 ORDER BY name")
        .map_err(AppError::from)?;
    let tpl_rows = stmt5
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(AppError::from)?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    for (tid, tname, tcat) in tpl_rows {
        templates.push(ExportTreeTemplate {
            id: tid,
            name: tname,
            category: tcat,
        });
    }

    Ok(ExportTree { clients, templates })
}

#[tauri::command]
pub fn create_export(
    state: State<AppState>,
    selection: ExportSelection,
    include_credential_values: bool,
    save_path: String,
) -> Result<ExportResult, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let clients_json = table_to_json(conn, "clients", Some(&selection.client_ids))?;
    let engagements_json = table_to_json(conn, "engagements", Some(&selection.engagement_ids))?;
    let findings_json = table_to_json(conn, "findings", Some(&selection.finding_ids))?;
    let mut credentials_json = table_to_json(conn, "credentials", Some(&selection.credential_ids))?;
    let documents_json = table_to_json(conn, "documents", Some(&selection.document_ids))?;
    let invoices_json = table_to_json(conn, "invoices", Some(&selection.invoice_ids))?;
    let templates_json = table_to_json(conn, "templates", Some(&selection.template_ids))?;

    if !include_credential_values {
        for cred in &mut credentials_json {
            if let Some(obj) = cred.as_object_mut() {
                obj.insert("value".to_string(), serde_json::Value::Null);
            }
        }
    }

    let manifest = serde_json::json!({
        "schema_version": "1.0.0",
        "app_version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().timestamp(),
        "entity_counts": {
            "clients": clients_json.len(),
            "engagements": engagements_json.len(),
            "findings": findings_json.len(),
            "credentials": credentials_json.len(),
            "documents": documents_json.len(),
            "invoices": invoices_json.len(),
            "templates": templates_json.len(),
        },
        "includes_credential_values": include_credential_values,
    });

    let mut counts = HashMap::<String, u32>::new();
    counts.insert("clients".to_string(), clients_json.len() as u32);
    counts.insert("engagements".to_string(), engagements_json.len() as u32);
    counts.insert("findings".to_string(), findings_json.len() as u32);
    counts.insert("credentials".to_string(), credentials_json.len() as u32);
    counts.insert("documents".to_string(), documents_json.len() as u32);
    counts.insert("invoices".to_string(), invoices_json.len() as u32);
    counts.insert("templates".to_string(), templates_json.len() as u32);

    let file = std::fs::File::create(&save_path).map_err(AppError::from)?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)
        .map_err(AppError::from)?;
    zip.write_all(manifest.to_string().as_bytes())
        .map_err(AppError::from)?;

    if !clients_json.is_empty() {
        zip.start_file("clients.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&clients_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !engagements_json.is_empty() {
        zip.start_file("engagements.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&engagements_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !findings_json.is_empty() {
        zip.start_file("findings.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&findings_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !credentials_json.is_empty() {
        zip.start_file("credentials.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&credentials_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !documents_json.is_empty() {
        zip.start_file("documents.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&documents_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !invoices_json.is_empty() {
        zip.start_file("invoices.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&invoices_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }
    if !templates_json.is_empty() {
        zip.start_file("templates.json", options)
            .map_err(AppError::from)?;
        zip.write_all(serde_json::to_string(&templates_json).unwrap().as_bytes())
            .map_err(AppError::from)?;
    }

    zip.finish().map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["system", "EXPORT", 0, "", "", format!("file_path={save_path}")],
    )
    .map_err(AppError::from)?;

    Ok(ExportResult {
        file_path: save_path,
        entity_counts: counts,
    })
}

#[tauri::command]
pub fn create_encrypted_export(
    state: State<AppState>,
    selection: ExportSelection,
    include_credential_values: bool,
    save_path: String,
    password: String,
) -> Result<ExportResult, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let clients_json = table_to_json(conn, "clients", Some(&selection.client_ids))?;
    let engagements_json = table_to_json(conn, "engagements", Some(&selection.engagement_ids))?;
    let findings_json = table_to_json(conn, "findings", Some(&selection.finding_ids))?;
    let mut credentials_json = table_to_json(conn, "credentials", Some(&selection.credential_ids))?;
    let documents_json = table_to_json(conn, "documents", Some(&selection.document_ids))?;
    let invoices_json = table_to_json(conn, "invoices", Some(&selection.invoice_ids))?;
    let templates_json = table_to_json(conn, "templates", Some(&selection.template_ids))?;

    if !include_credential_values {
        for cred in &mut credentials_json {
            if let Some(obj) = cred.as_object_mut() {
                obj.insert("value".to_string(), serde_json::Value::Null);
            }
        }
    }

    let manifest = serde_json::json!({
        "schema_version": "1.0.0",
        "app_version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().timestamp(),
        "encrypted": true,
        "encryption_method": "aes256gcm-argon2id",
        "entity_counts": {
            "clients": clients_json.len(),
            "engagements": engagements_json.len(),
            "findings": findings_json.len(),
            "credentials": credentials_json.len(),
            "documents": documents_json.len(),
            "invoices": invoices_json.len(),
            "templates": templates_json.len(),
        },
        "includes_credential_values": include_credential_values,
    });

    let mut zip_buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("manifest.json", options)
            .map_err(AppError::from)?;
        zip.write_all(manifest.to_string().as_bytes())
            .map_err(AppError::from)?;

        if !clients_json.is_empty() {
            zip.start_file("clients.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&clients_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !engagements_json.is_empty() {
            zip.start_file("engagements.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&engagements_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !findings_json.is_empty() {
            zip.start_file("findings.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&findings_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !credentials_json.is_empty() {
            zip.start_file("credentials.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&credentials_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !documents_json.is_empty() {
            zip.start_file("documents.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&documents_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !invoices_json.is_empty() {
            zip.start_file("invoices.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&invoices_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        if !templates_json.is_empty() {
            zip.start_file("templates.json", options)
                .map_err(AppError::from)?;
            zip.write_all(serde_json::to_string(&templates_json).unwrap().as_bytes())
                .map_err(AppError::from)?;
        }
        zip.finish().map_err(AppError::from)?;
    }

    let encrypted = encrypt_export_file(&zip_buf, &password)?;
    std::fs::write(&save_path, &encrypted).map_err(AppError::from)?;

    let mut counts = HashMap::<String, u32>::new();
    counts.insert("clients".to_string(), clients_json.len() as u32);
    counts.insert("engagements".to_string(), engagements_json.len() as u32);
    counts.insert("findings".to_string(), findings_json.len() as u32);
    counts.insert("credentials".to_string(), credentials_json.len() as u32);
    counts.insert("documents".to_string(), documents_json.len() as u32);
    counts.insert("invoices".to_string(), invoices_json.len() as u32);
    counts.insert("templates".to_string(), templates_json.len() as u32);

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["system", "EXPORT_ENCRYPTED", 0, "", "", format!("file_path={save_path}")],
    )
    .map_err(AppError::from)?;

    Ok(ExportResult {
        file_path: save_path,
        entity_counts: counts,
    })
}

#[tauri::command]
pub fn is_import_encrypted(file_path: String) -> Result<bool, String> {
    let data = std::fs::read(&file_path).map_err(AppError::from)?;
    Ok(is_encrypted_export_file(&data))
}

#[tauri::command]
pub fn decrypt_import_to_temp(file_path: String, password: String) -> Result<String, String> {
    let data = std::fs::read(&file_path).map_err(AppError::from)?;
    let plaintext = decrypt_export_file(&data, &password)?;
    let mut tmp = tempfile::Builder::new()
        .prefix("ss_import_")
        .suffix(".zip")
        .rand_bytes(12)
        .tempfile()
        .map_err(AppError::from)?;
    std::io::Write::write_all(&mut tmp, &plaintext).map_err(AppError::from)?;
    let path = tmp.into_temp_path();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn preview_import(state: State<AppState>, file_path: String) -> Result<ImportPreview, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let file = std::fs::File::open(&file_path).map_err(AppError::from)?;
    let mut archive =
        zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(AppError::from)?;

    let mut manifest_raw = String::new();
    archive
        .by_name("manifest.json")
        .map_err(AppError::from)?
        .read_to_string(&mut manifest_raw)
        .map_err(AppError::from)?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_raw).map_err(AppError::from)?;

    let version = manifest
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if version != "1.0.0" {
        return Ok(ImportPreview {
            compatible: false,
            conflicts: vec![ImportConflict {
                entity_type: "manifest".to_string(),
                import_name: format!("Unsupported version: {version}"),
            }],
        });
    }

    let mut conflicts: Vec<ImportConflict> = Vec::new();

    // Check client name conflicts
    if let Ok(mut clients_file) = archive.by_name("clients.json") {
        let mut clients_raw = String::new();
        clients_file.read_to_string(&mut clients_raw).ok();
        if let Ok(clients) = serde_json::from_str::<Vec<serde_json::Value>>(&clients_raw) {
            for client in clients {
                if let Some(name) = client.get("name").and_then(|n| n.as_str()) {
                    let exists: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM clients WHERE name = ? AND is_active = 1",
                            [name],
                            |row| row.get(0),
                        )
                        .unwrap_or(0);
                    if exists > 0 {
                        conflicts.push(ImportConflict {
                            entity_type: "client".to_string(),
                            import_name: name.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(ImportPreview {
        compatible: true,
        conflicts,
    })
}

#[tauri::command]
pub fn execute_import(
    state: State<AppState>,
    file_path: String,
    conflict_resolutions: Vec<ConflictResolution>,
) -> Result<ImportResult, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let file = std::fs::File::open(&file_path).map_err(AppError::from)?;
    let mut archive =
        zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(AppError::from)?;

    let mut imported = HashMap::<String, u32>::new();
    let mut skipped = HashMap::<String, u32>::new();

    // Build resolution maps
    let mut skip_set = std::collections::HashSet::<String>::new();
    let mut rename_map = std::collections::HashMap::<String, String>::new();
    let mut overwrite_set = std::collections::HashSet::<String>::new();
    for r in &conflict_resolutions {
        match r.action.as_str() {
            "skip" => {
                skip_set.insert(r.reference_key.clone());
            }
            "rename" => {
                rename_map.insert(
                    r.reference_key.clone(),
                    format!("{} (imported)", r.reference_key),
                );
            }
            "overwrite" => {
                overwrite_set.insert(r.reference_key.clone());
            }
            _ => {}
        }
    }

    // ── Import clients ──
    let mut client_id_map = std::collections::HashMap::<i64, i64>::new();
    if let Ok(mut clients_file) = archive.by_name("clients.json") {
        let mut clients_raw = String::new();
        clients_file.read_to_string(&mut clients_raw).ok();
        if let Ok(clients) = serde_json::from_str::<Vec<serde_json::Value>>(&clients_raw) {
            for client in clients {
                let name = client.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let key = format!("client::{name}");
                if skip_set.contains(&key) {
                    *skipped.entry("clients".to_string()).or_insert(0) += 1;
                    continue;
                }
                let final_name = rename_map.get(&key).map(|s| s.as_str()).unwrap_or(name);

                let old_id = client.get("id").and_then(|v| v.as_i64()).unwrap_or(0);

                // Overwrite: delete existing
                if overwrite_set.contains(&key) {
                    let _ = conn.execute("DELETE FROM clients WHERE name = ?", [final_name]);
                }

                conn.execute(
                    "INSERT INTO clients (name, contact_email, notes, tags, tech_stack, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        final_name,
                        client.get("contact_email").and_then(|v| v.as_str()).unwrap_or(""),
                        client.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
                        client.get("tags").and_then(|v| v.as_str()).unwrap_or("[]"),
                        client.get("tech_stack").and_then(|v| v.as_str()).unwrap_or("[]"),
                    ],
                ).map_err(AppError::from)?;
                let new_id = conn.last_insert_rowid();
                client_id_map.insert(old_id, new_id);
                *imported.entry("clients".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import engagements ──
    let mut engagement_id_map = std::collections::HashMap::<i64, i64>::new();
    if let Ok(mut engagements_file) = archive.by_name("engagements.json") {
        let mut raw = String::new();
        engagements_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                let old_id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let old_client_id = item.get("client_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let new_client_id = client_id_map
                    .get(&old_client_id)
                    .copied()
                    .unwrap_or(old_client_id);
                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");

                conn.execute(
                    "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model, engagement_type, status, start_date, end_date, scope_summary, notes, tags, payment_required, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        new_client_id,
                        name,
                        item.get("target_area").and_then(|v| v.as_str()).unwrap_or("Web"),
                        item.get("assessment_kind").and_then(|v| v.as_str()).unwrap_or("pentest"),
                        item.get("access_model").and_then(|v| v.as_str()).unwrap_or("remote"),
                        item.get("engagement_type").and_then(|v| v.as_str()).unwrap_or("one-time"),
                        item.get("status").and_then(|v| v.as_str()).unwrap_or("active"),
                        item.get("start_date").and_then(|v| v.as_str()),
                        item.get("end_date").and_then(|v| v.as_str()),
                        item.get("scope_summary").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("tags").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("payment_required").and_then(|v| v.as_bool()).unwrap_or(false),
                    ],
                ).map_err(AppError::from)?;
                let new_id = conn.last_insert_rowid();
                engagement_id_map.insert(old_id, new_id);
                *imported.entry("engagements".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import findings ──
    if let Ok(mut findings_file) = archive.by_name("findings.json") {
        let mut raw = String::new();
        findings_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                let old_eng_id = item
                    .get("engagement_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let new_eng_id = engagement_id_map
                    .get(&old_eng_id)
                    .copied()
                    .unwrap_or(old_eng_id);

                conn.execute(
                    "INSERT INTO findings (engagement_id, title, severity, overview, summary, affected_endpoints, evidence, impact_items, remediation_items, steps_to_reproduce, references_json, status, cvss_score, owasp_category, cwe_id, tags, notes, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        new_eng_id,
                        item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("severity").and_then(|v| v.as_str()).unwrap_or("medium"),
                        item.get("overview").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("summary").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("affected_endpoints").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("evidence").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("impact_items").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("remediation_items").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("steps_to_reproduce").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("references_json").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("status").and_then(|v| v.as_str()).unwrap_or("draft"),
                        item.get("cvss_score").and_then(|v| v.as_f64()),
                        item.get("owasp_category").and_then(|v| v.as_str()),
                        item.get("cwe_id").and_then(|v| v.as_str()),
                        item.get("tags").and_then(|v| v.as_str()).unwrap_or("[]"),
                        item.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
                    ],
                ).map_err(AppError::from)?;
                *imported.entry("findings".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import credentials ──
    if let Ok(mut creds_file) = archive.by_name("credentials.json") {
        let mut raw = String::new();
        creds_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                let old_eng_id = item
                    .get("engagement_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let new_eng_id = engagement_id_map
                    .get(&old_eng_id)
                    .copied()
                    .unwrap_or(old_eng_id);

                conn.execute(
                    "INSERT INTO credentials (engagement_id, label, username, value, credential_type, status, notes, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        new_eng_id,
                        item.get("label").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("username").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("value").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("credential_type").and_then(|v| v.as_str()).unwrap_or("password"),
                        item.get("status").and_then(|v| v.as_str()).unwrap_or("not_verified"),
                        item.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
                    ],
                ).map_err(AppError::from)?;
                *imported.entry("credentials".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import documents ──
    let mut document_id_map = std::collections::HashMap::<i64, i64>::new();
    if let Ok(mut docs_file) = archive.by_name("documents.json") {
        let mut raw = String::new();
        docs_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                let old_id = item.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let old_client_id = item.get("client_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let new_client_id = client_id_map
                    .get(&old_client_id)
                    .copied()
                    .unwrap_or(old_client_id);
                let old_eng_id = item.get("engagement_id").and_then(|v| v.as_i64());
                let new_eng_id = old_eng_id.and_then(|id| engagement_id_map.get(&id).copied());

                conn.execute(
                    "INSERT INTO documents (client_id, engagement_id, name, document_type, content, status, template_id, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        new_client_id,
                        new_eng_id,
                        item.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("document_type").and_then(|v| v.as_str()).unwrap_or("custom"),
                        item.get("content").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("status").and_then(|v| v.as_str()).unwrap_or("draft"),
                        item.get("template_id").and_then(|v| v.as_i64()),
                    ],
                ).map_err(AppError::from)?;
                let new_id = conn.last_insert_rowid();
                document_id_map.insert(old_id, new_id);
                *imported.entry("documents".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import invoices ──
    if let Ok(mut inv_file) = archive.by_name("invoices.json") {
        let mut raw = String::new();
        inv_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                let old_client_id = item.get("client_id").and_then(|v| v.as_i64()).unwrap_or(0);
                let new_client_id = client_id_map
                    .get(&old_client_id)
                    .copied()
                    .unwrap_or(old_client_id);
                let old_eng_id = item.get("engagement_id").and_then(|v| v.as_i64());
                let new_eng_id = old_eng_id.and_then(|id| engagement_id_map.get(&id).copied());

                conn.execute(
                    "INSERT INTO invoices (client_id, engagement_id, invoice_number, document_type, currency, tax_rate_bps, discount_type, discount_value, notes, status, due_date, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        new_client_id,
                        new_eng_id,
                        item.get("invoice_number").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("document_type").and_then(|v| v.as_str()).unwrap_or("invoice"),
                        item.get("currency").and_then(|v| v.as_str()).unwrap_or("USD"),
                        item.get("tax_rate_bps").and_then(|v| v.as_i64()).unwrap_or(0),
                        item.get("discount_type").and_then(|v| v.as_str()).unwrap_or("none"),
                        item.get("discount_value").and_then(|v| v.as_i64()).unwrap_or(0),
                        item.get("notes").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("status").and_then(|v| v.as_str()).unwrap_or("draft"),
                        item.get("due_date").and_then(|v| v.as_str()),
                    ],
                ).map_err(AppError::from)?;
                let invoice_id = conn.last_insert_rowid();

                // Import invoice items
                if let Some(items_json) = item.get("items").and_then(|v| v.as_array()).cloned() {
                    for ii in items_json {
                        conn.execute(
                            "INSERT INTO invoice_items (invoice_id, description, quantity, rate_cents, amount, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, strftime('%s', 'now'))",
                            rusqlite::params![
                                invoice_id,
                                ii.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                                ii.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1),
                                ii.get("rate_cents").and_then(|v| v.as_i64()).unwrap_or(0),
                                ii.get("amount").and_then(|v| v.as_i64()).unwrap_or(0),
                            ],
                        ).map_err(AppError::from)?;
                    }
                }
                *imported.entry("invoices".to_string()).or_insert(0) += 1;
            }
        }
    }

    // ── Import templates ──
    if let Ok(mut tpl_file) = archive.by_name("templates.json") {
        let mut raw = String::new();
        tpl_file.read_to_string(&mut raw).ok();
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
            for item in items {
                conn.execute(
                    "INSERT INTO templates (name, category, subcategory, content, tags, is_builtin, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                    rusqlite::params![
                        item.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("category").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("subcategory").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("content").and_then(|v| v.as_str()).unwrap_or(""),
                        item.get("tags").and_then(|v| v.as_str()).unwrap_or("[]"),
                    ],
                ).map_err(AppError::from)?;
                *imported.entry("templates".to_string()).or_insert(0) += 1;
            }
        }
    }

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["system", "IMPORT", 0, "", "", format!("file_path={file_path}")],
    )
    .map_err(AppError::from)?;

    Ok(ImportResult { imported, skipped })
}

#[tauri::command]
pub fn export_vault_json(
    state: State<AppState>,
    file_path: String,
) -> Result<ExportSummary, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;

    let mut export = serde_json::Map::<String, serde_json::Value>::new();
    export.insert("version".to_string(), "1.0".into());
    export.insert(
        "exported_at".to_string(),
        chrono::Utc::now().to_rfc3339().into(),
    );

    let clients = table_to_json(conn, "clients", None)?;
    let engagements = table_to_json(conn, "engagements", None)?;
    let findings = table_to_json(conn, "findings", None)?;
    let credentials = table_to_json(conn, "credentials", None)?;
    let documents = table_to_json(conn, "documents", None)?;
    let invoices = table_to_json(conn, "invoices", None)?;
    let templates = table_to_json(conn, "templates", None)?;
    let reports = table_to_json(conn, "reports", None)?;

    export.insert("clients".to_string(), clients.clone().into());
    export.insert("engagements".to_string(), engagements.clone().into());
    export.insert("findings".to_string(), findings.clone().into());
    export.insert("credentials".to_string(), credentials.clone().into());
    export.insert("documents".to_string(), documents.clone().into());
    export.insert("invoices".to_string(), invoices.clone().into());
    export.insert("templates".to_string(), templates.clone().into());
    export.insert("reports".to_string(), reports.clone().into());

    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())?;

    let summary = ExportSummary {
        clients: clients.len() as u32,
        engagements: engagements.len() as u32,
        findings: findings.len() as u32,
        credentials: credentials.len() as u32,
        documents: documents.len() as u32,
        invoices: invoices.len() as u32,
        templates: templates.len() as u32,
        reports: reports.len() as u32,
        file_path,
    };

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["system", "EXPORT_JSON", 0, "", "", format!("file_path={}", summary.file_path)],
    )
    .map_err(AppError::from)?;

    Ok(summary)
}

// ─────────────────────────────────────────────────────────────
// Encrypted export helpers (PROP-026)
// ─────────────────────────────────────────────────────────────

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};

const EXPORT_MAGIC: &[u8] = b"SSENC\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

fn derive_export_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(AppError::from)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
        .map_err(AppError::from)?;
    Ok(key_bytes)
}

pub fn encrypt_export_file(plaintext: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let salt: [u8; SALT_LEN] = rand::random();
    let nonce_bytes: [u8; NONCE_LEN] = rand::random();

    let key_bytes = derive_export_key(password, &salt)?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(AppError::from)?;

    let mut out = Vec::with_capacity(EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(EXPORT_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_export_file(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if data.len() < EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN + TAG_LEN {
        return Err("File too small to be a valid encrypted export.".to_string());
    }
    if &data[0..EXPORT_MAGIC.len()] != EXPORT_MAGIC {
        return Err("Not an encrypted export file (invalid magic bytes).".to_string());
    }
    let salt = &data[EXPORT_MAGIC.len()..EXPORT_MAGIC.len() + SALT_LEN];
    let nonce_bytes =
        &data[EXPORT_MAGIC.len() + SALT_LEN..EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN];
    let ciphertext = &data[EXPORT_MAGIC.len() + SALT_LEN + NONCE_LEN..];

    let key_bytes = derive_export_key(password, salt)?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Invalid export password or corrupted file.".to_string())?;
    Ok(plaintext)
}

pub fn is_encrypted_export_file(data: &[u8]) -> bool {
    data.len() >= EXPORT_MAGIC.len() && &data[0..EXPORT_MAGIC.len()] == EXPORT_MAGIC
}

#[allow(dead_code)]
fn read_file_or_decrypt(file_path: &str, password: Option<&str>) -> Result<Vec<u8>, String> {
    let data = std::fs::read(file_path).map_err(AppError::from)?;
    if is_encrypted_export_file(&data) {
        let pw = password.ok_or_else(|| {
            "This export is encrypted. Please provide the export password.".to_string()
        })?;
        decrypt_export_file(&data, pw)
    } else {
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let password = "correct horse battery staple";
        let encrypted = encrypt_export_file(plaintext, password).unwrap();
        let decrypted = decrypt_export_file(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_password_rejected() {
        let plaintext = b"vault contents here";
        let encrypted = encrypt_export_file(plaintext, "right").unwrap();
        let err = decrypt_export_file(&encrypted, "wrong").unwrap_err();
        assert!(err.to_lowercase().contains("invalid") || err.to_lowercase().contains("corrupt"));
    }

    #[test]
    fn test_corrupt_file_rejected() {
        let mut encrypted = encrypt_export_file(b"hello", "pw").unwrap();
        // Truncate the file
        encrypted.truncate(encrypted.len() / 2);
        let err = decrypt_export_file(&encrypted, "pw").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn test_invalid_magic_rejected() {
        // Length must be at least 34 bytes (magic+ver+salt+nonce+tag)
        // and the leading bytes must NOT be the SSENC magic.
        let mut garbage = b"NOTANEXPORTFILE".to_vec();
        garbage.extend(std::iter::repeat(b'X').take(64));
        let err = decrypt_export_file(&garbage, "pw").unwrap_err();
        assert!(
            err.to_lowercase().contains("not an encrypted"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_is_encrypted_export_detection() {
        let plain = b"PK\x03\x04plain zip content";
        assert!(!is_encrypted_export_file(plain));

        let encrypted = encrypt_export_file(plain, "pw").unwrap();
        assert!(is_encrypted_export_file(&encrypted));

        // Truncated file shorter than the magic prefix is not detected.
        let short = &encrypted[..3];
        assert!(!is_encrypted_export_file(short));
    }

    #[test]
    fn test_encrypted_output_has_expected_layout() {
        let plaintext = b"some bytes";
        let encrypted = encrypt_export_file(plaintext, "pw").unwrap();
        // Magic (5) + version (1) + salt (16) + nonce (12) + ciphertext + tag (16)
        assert!(encrypted.len() >= 5 + 1 + 16 + 12 + plaintext.len() + 16);
        assert_eq!(&encrypted[0..5], b"SSENC");
        assert_eq!(encrypted[5], 0x01);
    }

    #[test]
    fn test_empty_plaintext_roundtrip() {
        let encrypted = encrypt_export_file(b"", "pw").unwrap();
        let decrypted = decrypt_export_file(&encrypted, "pw").unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn test_different_salts_per_encryption() {
        let plaintext = b"identical plaintext";
        let e1 = encrypt_export_file(plaintext, "same").unwrap();
        let e2 = encrypt_export_file(plaintext, "same").unwrap();
        // Salt and nonce should be random, so ciphertexts differ
        assert_ne!(e1, e2);
        // But both decrypt to the same plaintext
        assert_eq!(decrypt_export_file(&e1, "same").unwrap(), plaintext);
        assert_eq!(decrypt_export_file(&e2, "same").unwrap(), plaintext);
    }
}
