//! Findings import — parse scanner output (Nessus XML, CSV) into SecuritySmith findings.

use std::collections::HashSet;

use crate::WorkspaceError;

/// A parsed finding from a scanner output file.
#[derive(Debug, Clone)]
pub struct ParsedFinding {
    pub title: String,
    pub severity: String,
    pub description: String,
}

/// Import summary returned after importing findings.
#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub parsed: usize,
    pub created: usize,
    pub duplicates: usize,
}

/// Map Nessus severity (0-4) to SecuritySmith severity.
fn map_nessus_severity(severity: &str) -> String {
    match severity {
        "4" => "critical",
        "3" => "high",
        "2" => "medium",
        "1" => "low",
        "0" => "informational",
        _ => "informational",
    }
    .to_string()
}

/// Parse Nessus XML (.nessus) file content.
///
/// quick-xml is inherently XXE-safe — it does not process DTDs or external entities.
/// Entity expansion is bounded by the parser's internal limits.
pub fn parse_nessus(content: &str) -> Result<Vec<ParsedFinding>, String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut findings = Vec::new();
    let mut buf = Vec::new();
    let mut in_report_item = false;
    let mut in_description = false;
    let mut current_title = String::new();
    let mut current_severity = String::new();
    let mut current_description = String::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"ReportItem" => {
                        in_report_item = true;
                        current_title.clear();
                        current_severity.clear();
                        current_description.clear();
                        // Get severity from attributes
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"severity" {
                                current_severity =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                            }
                            if attr.key.as_ref() == b"pluginName" {
                                current_title =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                            }
                        }
                    }
                    b"description" if in_report_item => {
                        in_description = true;
                        text_buf.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"ReportItem" if in_report_item => {
                    in_report_item = false;
                    if !current_title.is_empty() {
                        findings.push(ParsedFinding {
                            title: current_title.clone(),
                            severity: map_nessus_severity(&current_severity),
                            description: current_description.clone(),
                        });
                    }
                }
                b"description" if in_description => {
                    in_description = false;
                    current_description = text_buf.trim().to_string();
                }
                _ => {}
            },
            Ok(Event::Text(e)) if in_description => {
                text_buf.push_str(&String::from_utf8_lossy(&e[..]));
            }
            Ok(Event::CData(e)) if in_description => {
                text_buf.push_str(&String::from_utf8_lossy(&e[..]));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                ));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(findings)
}

/// Parse CSV file content with configurable title and severity columns.
pub fn parse_csv(
    content: &str,
    title_col: usize,
    severity_col: usize,
) -> Result<Vec<ParsedFinding>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| format!("CSV header error: {e}"))?
        .clone();

    let mut findings = Vec::new();
    let max_col = title_col.max(severity_col);

    for (i, record) in reader.records().enumerate() {
        let record = record.map_err(|e| format!("CSV parse error at row {}: {e}", i + 2))?;

        if record.len() <= max_col {
            continue;
        }

        let title = record.get(title_col).unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }

        let severity_raw = record.get(severity_col).unwrap_or("").trim().to_lowercase();
        let severity = normalize_severity(&severity_raw);

        let description = record
            .iter()
            .enumerate()
            .map(|(j, v)| {
                if j < headers.len() {
                    format!("**{}**: {}", headers.get(j).unwrap_or(""), v)
                } else {
                    v.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        findings.push(ParsedFinding {
            title,
            severity,
            description,
        });
    }

    Ok(findings)
}

/// Normalize severity string to SecuritySmith's 5-level scale.
fn normalize_severity(s: &str) -> String {
    match s {
        "critical" | "crit" | "4" => "critical",
        "high" | "3" => "high",
        "medium" | "med" | "moderate" | "2" => "medium",
        "low" | "1" => "low",
        "informational" | "info" | "0" => "informational",
        _ => "informational",
    }
    .to_string()
}

/// Import parsed findings into an engagement.
/// Creates Markdown finding files with frontmatter. Skips duplicates by title.
pub fn import_findings(
    ws: &crate::Workspace,
    engagement_path: &str,
    parsed: Vec<ParsedFinding>,
) -> Result<ImportSummary, WorkspaceError> {
    let (eng_dir, entity_type) = crate::entities::resolve_existing_entity(ws, engagement_path)?;
    if entity_type != crate::entities::EntityType::Engagement {
        return Err(WorkspaceError::NotFound(eng_dir));
    }

    let findings_dir = eng_dir.join("findings");
    std::fs::create_dir_all(findings_dir.as_std_path())?;

    // Get existing finding titles for duplicate detection
    let existing = crate::findings::list_findings(&eng_dir, None, None)?;
    let existing_titles: HashSet<String> = existing
        .iter()
        .map(|f| {
            // Read the finding file to get the title (first heading or frontmatter)
            let content =
                std::fs::read_to_string(findings_dir.join(&f.filename)).unwrap_or_default();
            // Extract title from the first H1 in the body
            content
                .lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").to_string())
                .unwrap_or_default()
        })
        .collect();

    // Get the project path for ID generation
    let segments: Vec<&str> = engagement_path.split('/').collect();
    let project_path = if segments.len() >= 2 {
        format!("{}/{}", segments[0], segments[1])
    } else {
        engagement_path.to_string()
    };

    let parsed_count = parsed.len();
    let mut created = 0;
    let mut duplicates = 0;

    for finding in parsed {
        if existing_titles.contains(&finding.title) {
            duplicates += 1;
            continue;
        }

        // Generate finding ID using the project's sequence
        let (proj_dir, _) = crate::entities::resolve_existing_entity(ws, &project_path)?;
        let id = crate::entities::increment_sequence(&proj_dir)?;

        let slug: String = finding
            .title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_string();

        let filename = format!("{}_{}.md", id.to_lowercase(), slug);
        let filepath = findings_dir.join(&filename);

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = format!(
            "---\nid: \"{}\"\nstatus: \"open\"\nseverity: \"{}\"\ncreated: \"{}\"\nupdated: \"{}\"\n---\n\n# {}\n\n{}\n",
            id, finding.severity, today, today, finding.title, finding.description
        );

        crate::atomic_write(&filepath, content.as_bytes())?;
        created += 1;
    }

    Ok(ImportSummary {
        parsed: parsed_count,
        created,
        duplicates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn parse_nessus_basic() {
        let xml = r#"<?xml version="1.0"?>
<NessusClientData_v2>
  <Report>
    <ReportHost name="192.168.1.1">
      <HostProperties>
        <tag name="host-fqdn">test.local</tag>
      </HostProperties>
      <ReportItem port="443" svc_name="www" protocol="tcp" severity="3" pluginID="12345" pluginName="SSL Certificate Weak">
        <description>The SSL certificate uses weak ciphers.</description>
      </ReportItem>
      <ReportItem port="80" svc_name="www" protocol="tcp" severity="2" pluginID="12346" pluginName="HTTP Methods">
        <description>HTTP methods allowed.</description>
      </ReportItem>
    </ReportHost>
  </Report>
</NessusClientData_v2>"#;
        let findings = parse_nessus(xml).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].title, "SSL Certificate Weak");
        assert_eq!(findings[0].severity, "high");
        assert!(findings[0].description.contains("weak ciphers"));
        assert_eq!(findings[1].severity, "medium");
    }

    #[test]
    fn parse_nessus_empty() {
        let xml = r#"<?xml version="1.0"?><NessusClientData_v2></NessusClientData_v2>"#;
        let findings = parse_nessus(xml).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn parse_nessus_malformed() {
        let xml = "not xml at all";
        let result = parse_nessus(xml);
        // quick-xml is lenient — it won't error on text, just won't find findings
        // The important thing is it doesn't panic
        assert!(result.is_ok());
    }

    #[test]
    fn parse_csv_basic() {
        let csv_content = "title,severity,description\n\
            SQL Injection,high,Found SQL injection in login form\n\
            XSS,medium,Reflected XSS in search parameter";
        let findings = parse_csv(csv_content, 0, 1).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].title, "SQL Injection");
        assert_eq!(findings[0].severity, "high");
    }

    #[test]
    fn parse_csv_severity_normalization() {
        let csv_content = "title,severity\n\
            Test1,crit\n\
            Test2,3\n\
            Test3,moderate";
        let findings = parse_csv(csv_content, 0, 1).unwrap();
        assert_eq!(findings[0].severity, "critical");
        assert_eq!(findings[1].severity, "high");
        assert_eq!(findings[2].severity, "medium");
    }

    #[test]
    fn parse_csv_skips_empty_titles() {
        let csv_content = "title,severity\n\
            ,high\n\
            Valid,low";
        let findings = parse_csv(csv_content, 0, 1).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "Valid");
    }

    #[test]
    fn import_creates_findings() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        let parsed = vec![
            ParsedFinding {
                title: "SQL Injection".to_string(),
                severity: "high".to_string(),
                description: "Found SQLi".to_string(),
            },
            ParsedFinding {
                title: "XSS".to_string(),
                severity: "medium".to_string(),
                description: "Found XSS".to_string(),
            },
        ];

        let summary = import_findings(&ws, "acme/web/initial", parsed).unwrap();
        assert_eq!(summary.parsed, 2);
        assert_eq!(summary.created, 2);
        assert_eq!(summary.duplicates, 0);

        let (eng_dir, _) =
            crate::entities::resolve_existing_entity(&ws, "acme/web/initial").unwrap();
        let findings = crate::findings::list_findings(&eng_dir, None, None).unwrap();
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn import_skips_duplicates() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        // Create an existing finding
        tw.create_finding(&eng, "ACME-WEB-001", "SQL Injection");

        let parsed = vec![ParsedFinding {
            title: "SQL Injection".to_string(),
            severity: "high".to_string(),
            description: "Duplicate".to_string(),
        }];

        let summary = import_findings(&ws, "acme/web/initial", parsed).unwrap();
        assert_eq!(summary.parsed, 1);
        assert_eq!(summary.created, 0);
        assert_eq!(summary.duplicates, 1);
    }
}
