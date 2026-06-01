use crate::parsers::{
    CsvColumnMapping, FindingParser, ImportPreview, ImportResult, ParsedFinding, burp::BurpParser,
    csv_import::CsvParser, nessus::NessusParser, nmap::NmapParser, nuclei::NucleiParser,
    zap::ZapJsonParser,
};
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use tauri::State;

const MAX_IMPORT_BYTES: u64 = 100 * 1024 * 1024;

fn do_check_duplicates(
    conn: &Connection,
    engagement_id: u32,
    findings: &mut [ParsedFinding],
) -> Result<u32, String> {
    let mut stmt = conn
        .prepare(
            "SELECT title, affected_endpoints FROM findings WHERE engagement_id = ? AND is_active = 1",
        )
        .map_err(|e| format!("Failed to prepare duplicate check: {}", e))?;
    let rows = stmt
        .query_map(params![engagement_id], |row| {
            let title: String = row.get(0)?;
            let eps: String = row.get(1)?;
            Ok((title, eps))
        })
        .map_err(|e| format!("Failed to query duplicates: {}", e))?;

    let mut existing_titles: Vec<(String, Vec<String>)> = Vec::new();
    for row in rows {
        let (title, eps_str) = row.map_err(|e| format!("Row error: {}", e))?;
        let eps: Vec<serde_json::Value> = serde_json::from_str(&eps_str).unwrap_or_default();
        let paths: Vec<String> = eps
            .iter()
            .filter_map(|v| v.get("path").and_then(|p| p.as_str()).map(String::from))
            .collect();
        existing_titles.push((title, paths));
    }

    let mut dup_count = 0u32;
    for finding in findings.iter_mut() {
        for (title, paths) in &existing_titles {
            if title.eq_ignore_ascii_case(&finding.title) {
                for ep in &finding.affected_endpoints {
                    if paths.contains(&ep.path) {
                        finding.is_duplicate = true;
                        dup_count += 1;
                        break;
                    }
                }
            }
        }
    }
    Ok(dup_count)
}

fn do_parse_file(
    conn: &Connection,
    file_path: &str,
    format: &str,
    engagement_id: u32,
    csv_mapping: Option<CsvColumnMapping>,
) -> Result<ImportPreview, String> {
    let meta = std::fs::metadata(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    if meta.len() == 0 {
        return Err("Import file is empty.".to_string());
    }
    if meta.len() > MAX_IMPORT_BYTES {
        return Err("Import file exceeds maximum size of 100MB.".to_string());
    }

    let content = std::fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut findings: Vec<ParsedFinding> = match format {
        "nessus" => NessusParser.parse(&content),
        "burp" => BurpParser.parse(&content),
        "zap_json" => ZapJsonParser.parse(&content),
        "nmap" => NmapParser.parse(&content),
        "nuclei" => NucleiParser.parse(&content),
        "csv" => {
            let mapping = csv_mapping.ok_or("CSV mapping is required for CSV import.")?;
            CsvParser { mapping }.parse(&content)
        }
        _ => return Err("Unsupported import format.".to_string()),
    }
    .map_err(|e| format!("Parse error: {}", e))?;

    // engagement exists and is active
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM engagements WHERE id = ? AND is_active = 1",
            params![engagement_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Failed to check engagement: {}", e))?
        .unwrap_or(false);

    if !exists {
        return Err("Engagement not found or has been archived.".to_string());
    }

    let dup_count = do_check_duplicates(conn, engagement_id, &mut findings)?;
    let total = findings.len() as u32;

    Ok(ImportPreview {
        findings,
        total_parsed: total,
        duplicates_found: dup_count,
        format: format.to_string(),
    })
}

fn do_commit_import(
    conn: &Connection,
    engagement_id: u32,
    findings: Vec<ParsedFinding>,
) -> Result<ImportResult, String> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM engagements WHERE id = ? AND is_active = 1",
            params![engagement_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Failed to check engagement: {}", e))?
        .unwrap_or(false);

    if !exists {
        return Err("Engagement not found or has been archived.".to_string());
    }

    let mut imported = 0u32;
    let mut skipped = 0u32;

    let eps_json = serde_json::to_string(&Vec::<&str>::new())
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let impact_json = serde_json::to_string(&Vec::<&str>::new())
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let _refs_json = serde_json::to_string(&Vec::<&str>::new())
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let tags_json = serde_json::to_string(&Vec::<&str>::new())
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    let eps_empty = serde_json::to_string(&Vec::<&str>::new()).unwrap_or_default();
    let _impact_empty = eps_empty.clone();
    let _refs_empty = eps_empty.clone();

    for pf in findings {
        if pf.is_duplicate {
            skipped += 1;
            continue;
        }

        let affected = serde_json::to_string(&pf.affected_endpoints)
            .map_err(|e| format!("Failed to serialize endpoints: {}", e))?;
        let evidence = eps_json.clone();
        let impact = impact_json.clone();
        let remediation = serde_json::to_string(&pf.remediation_items)
            .map_err(|e| format!("Failed to serialize remediation: {}", e))?;
        let references = serde_json::to_string(&pf.references)
            .map_err(|e| format!("Failed to serialize references: {}", e))?;

        conn.execute(
            "INSERT INTO findings (
                engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, references_json, status, tags, is_active,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'draft', ?12, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
            params![
                engagement_id,
                pf.title.chars().take(500).collect::<String>(),
                pf.severity,
                pf.overview.chars().take(1000).collect::<String>(),
                pf.summary.chars().take(50000).collect::<String>(),
                affected,
                evidence,
                impact,
                remediation,
                "Imported from " .to_string() + &pf.source_tool,
                references,
                tags_json.clone(),
            ],
        )
        .map_err(|e| format!("Failed to insert finding: {}", e))?;

        imported += 1;
    }

    let context = format!(
        "import_format={}, engagement_id={}, imported={}, skipped={}",
        "import", engagement_id, imported, skipped
    );
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["findings", "IMPORT", engagement_id, "", "", context],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(ImportResult {
        imported_count: imported,
        skipped_count: skipped,
    })
}

// ─────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn parse_import_file(
    state: State<AppState>,
    file_path: String,
    format: String,
    engagement_id: u32,
    csv_mapping: Option<CsvColumnMapping>,
) -> Result<ImportPreview, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_parse_file(conn, &file_path, &format, engagement_id, csv_mapping)
}

#[tauri::command]
pub fn commit_import(
    state: State<AppState>,
    engagement_id: u32,
    findings: Vec<ParsedFinding>,
) -> Result<ImportResult, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_commit_import(conn, engagement_id, findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[allow(dead_code)]
    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let key = [99u8; 32];
        let conn = db::open_vault(tmp.path(), &key).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_nessus_parsing() {
        let xml = r#"<?xml version="1.0"?>
<NessusClientData>
<Report>
<ReportItem port="443" pluginName="SSL Weak Cipher" severity="2">
<plugin_output>Weak ciphers supported</plugin_output>
<solution>Disable weak ciphers</solution>
<synopsis>The remote server supports weak SSL ciphers.</synopsis>
</ReportItem>
</Report>
</NessusClientData>
"#;
        let findings = NessusParser.parse(xml.as_bytes()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "SSL Weak Cipher");
        assert_eq!(findings[0].severity, "medium");
    }

    #[test]
    fn test_burp_parsing() {
        let xml = r#"<issues>
<issue>
<name>XSS</name>
<severity>High</severity>
<host>https://example.com</host>
<path>/search</path>
<issueBackground>Reflected XSS found.</issueBackground>
<issueDetail>The search parameter reflects input.</issueDetail>
</issue>
</issues>
"#;
        let findings = BurpParser.parse(xml.as_bytes()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "XSS");
        assert_eq!(findings[0].severity, "high");
    }

    #[test]
    fn test_nmap_parsing() {
        let xml = r#"<?xml version="1.0"?>
<nmaprun>
<host>
<address addr="192.168.1.1"/>
<ports>
<port portid="80" protocol="tcp">
<state state="open"/>
</port>
</ports>
</host>
</nmaprun>
"#;
        let findings = NmapParser.parse(xml.as_bytes()).unwrap();
        assert_eq!(findings.len(), 1);
        assert!(findings[0].title.contains("80/tcp"));
        assert_eq!(findings[0].severity, "informational");
    }

    #[test]
    fn test_nuclei_parsing() {
        let line = r#"{"template-id":"test","info":{"name":"Test vuln","severity":"high","description":"desc"},"host":"https://target.com","matched-at":"/api/v1"}"#;
        let findings = NucleiParser.parse(line.as_bytes()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, "high");
    }

    #[test]
    fn test_csv_parsing() {
        let csv = "Title,Severity,Description\nXSS,high,Reflected XSS\nSQLi,critical,Injection\n";
        let mapping = CsvColumnMapping {
            title: 0,
            severity: 1,
            description: Some(2),
            remediation: None,
            affected_url: None,
        };
        let findings = CsvParser { mapping }.parse(csv.as_bytes()).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, "high");
    }
}
