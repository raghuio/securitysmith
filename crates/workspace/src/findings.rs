//! Finding model — create, show, update, list, remove, gather for export.
//!
//! Findings are Markdown files with YAML frontmatter (id, status, severity, created, updated).
//! All other content is in the Markdown body. The tool does not parse body content.

use camino::Utf8PathBuf;
use std::fs;

use crate::entities;
use crate::{Workspace, WorkspaceError};

/// Create a new finding under an engagement.
///
/// Generates ID (client prefix + project abbreviation + project sequence),
/// writes frontmatter + optional template body, returns the file path.
pub fn create_finding(
    ws: &Workspace,
    engagement_path: &str,
    title: &str,
    no_template: bool,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let segments: Vec<&str> = engagement_path.split('/').collect();
    let (engagement_dir, entity_type) = entities::resolve_existing_entity(ws, engagement_path)?;
    if entity_type != entities::EntityType::Engagement {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(engagement_path)));
    }

    let client_dir = ws.root.join(segments[0]);
    let project_dir = client_dir.join(segments[1]);

    // Get client prefix and project abbreviation
    let prefix = entities::get_client_prefix(&client_dir)?;
    let abbr = entities::get_project_abbreviation(&project_dir)?;

    // Increment sequence and build ID
    let seq = entities::increment_sequence(&project_dir)?;
    let finding_id = format!("{}-{}-{}", prefix, abbr, seq);

    // Create findings/ directory if needed
    let findings_dir = engagement_dir.join("findings");
    fs::create_dir_all(&findings_dir)?;

    // Build filename: <id_lowercase_with_underscores>_<slug>.md
    let slug = crate::slug_from_title(title);
    let filename_id = finding_id.to_lowercase().replace('-', "_");
    let filename = format!("{}_{}.md", filename_id, slug);
    let file_path = findings_dir.join(&filename);

    // Get effective defaults from config inheritance (project > client > workspace > built-in)
    let defaults = crate::entities::get_effective_defaults(ws, segments[0], Some(segments[1]));

    // Build frontmatter
    let today = crate::utc_today();
    let mut fm = serde_yaml::Mapping::new();
    fm.insert(
        serde_yaml::Value::String("id".into()),
        serde_yaml::Value::String(finding_id.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("status".into()),
        serde_yaml::Value::String(defaults.status.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("severity".into()),
        serde_yaml::Value::String(defaults.severity.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("created".into()),
        serde_yaml::Value::String(today.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("updated".into()),
        serde_yaml::Value::String(today),
    );

    // Build body from template or bare
    let body = if no_template {
        format!("# {}\n\n", title)
    } else {
        match crate::templates::get_template(ws, "finding")? {
            Some(t) => t.replace("{{title}}", title),
            None => format!("# {}\n\n", title),
        }
    };

    ss_frontmatter::write_file(
        file_path.as_std_path(),
        &serde_yaml::Value::Mapping(fm),
        &body,
    )?;

    Ok(file_path)
}

/// Find a finding file by its ID across the workspace.
pub fn find_finding_file(ws: &Workspace, finding_id: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    let target_lower = finding_id.to_lowercase();
    let target_lower = target_lower.replace('-', "_");

    for client_entry in fs::read_dir(&ws.root)? {
        let client_entry = client_entry?;
        if !client_entry.file_type()?.is_dir() {
            continue;
        }
        let client_dir = client_entry.path();
        if !client_dir.join("config.toml").exists() {
            continue;
        }
        for project_entry in fs::read_dir(&client_dir)? {
            let project_entry = project_entry?;
            if !project_entry.file_type()?.is_dir() {
                continue;
            }
            let project_dir = project_entry.path();
            if !project_dir.join("config.toml").exists() {
                continue;
            }
            for eng_entry in fs::read_dir(&project_dir)? {
                let eng_entry = eng_entry?;
                if !eng_entry.file_type()?.is_dir() {
                    continue;
                }
                let eng_dir = eng_entry.path();
                if !eng_dir.join("config.toml").exists() {
                    continue;
                }
                let findings_dir = eng_dir.join("findings");
                if !findings_dir.exists() {
                    continue;
                }
                for f_entry in fs::read_dir(&findings_dir)? {
                    let f_entry = f_entry?;
                    let fname = f_entry.file_name();
                    let fname = fname.to_string_lossy();
                    if fname.starts_with(&target_lower) && fname.ends_with(".md") {
                        return Ok(Utf8PathBuf::from_path_buf(f_entry.path()).unwrap_or_default());
                    }
                }
            }
        }
    }
    Err(WorkspaceError::NotFound(Utf8PathBuf::from(finding_id)))
}

/// Show finding content (frontmatter + body).
pub fn show_finding(ws: &Workspace, finding_id: &str) -> Result<String, WorkspaceError> {
    let path = find_finding_file(ws, finding_id)?;
    Ok(fs::read_to_string(&path)?)
}

/// Update a finding's status in frontmatter.
pub fn update_finding_status(
    ws: &Workspace,
    finding_id: &str,
    status: &str,
) -> Result<(), WorkspaceError> {
    let path = find_finding_file(ws, finding_id)?;

    ss_frontmatter::update_field(
        path.as_std_path(),
        "status",
        &serde_yaml::Value::String(status.to_string()),
    )?;
    Ok(())
}

/// Update a finding's severity in frontmatter.
pub fn update_finding_severity(
    ws: &Workspace,
    finding_id: &str,
    severity: &str,
) -> Result<(), WorkspaceError> {
    let path = find_finding_file(ws, finding_id)?;
    ss_frontmatter::update_field(
        path.as_std_path(),
        "severity",
        &serde_yaml::Value::String(severity.to_string()),
    )?;
    Ok(())
}

/// Valid client response values.
pub const VALID_CLIENT_RESPONSES: &[&str] = &[
    "acknowledged",
    "in_progress",
    "fixed",
    "accepted_risk",
    "disputed",
    "deferred",
    "no_response",
];

/// Valid retest result values.
pub const VALID_RETEST_RESULTS: &[&str] = &["not_tested", "pass", "fail", "partial"];

/// Update a remediation field in finding frontmatter.
pub fn update_remediation_field(
    ws: &Workspace,
    finding_id: &str,
    field: &str,
    value: &str,
) -> Result<(), WorkspaceError> {
    let path = find_finding_file(ws, finding_id)?;
    ss_frontmatter::update_field(
        path.as_std_path(),
        field,
        &serde_yaml::Value::String(value.to_string()),
    )?;
    Ok(())
}

/// Calculate fix deadline from severity.
/// critical=30d, high=60d, medium=90d, low=180d, informational=none
pub fn calculate_fix_deadline(severity: &str) -> String {
    let days = match severity {
        "critical" => 30,
        "high" => 60,
        "medium" => 90,
        "low" => 180,
        _ => return String::new(),
    };
    let today = chrono::Utc::now();
    let deadline = today + chrono::Duration::days(days);
    deadline.format("%Y-%m-%d").to_string()
}

/// A finding entry from listing.
#[derive(Debug, Clone)]
pub struct FindingEntry {
    pub id: String,
    pub status: String,
    pub severity: String,
    pub filename: String,
    pub path: Utf8PathBuf,
}

/// List findings in an engagement directory, optionally filtered by severity and/or status.
pub fn list_findings(
    engagement_dir: &Utf8PathBuf,
    severity_filter: Option<&str>,
    status_filter: Option<&str>,
) -> Result<Vec<FindingEntry>, WorkspaceError> {
    let findings_dir = engagement_dir.join("findings");
    if !findings_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for f_entry in fs::read_dir(&findings_dir)? {
        let f_entry = f_entry?;
        let path = f_entry.path();
        let fname = f_entry.file_name().to_string_lossy().to_string();
        if !fname.ends_with(".md") {
            continue;
        }

        let parsed = ss_frontmatter::parse_file(&path)?;
        let fm = &parsed.frontmatter;
        let id = fm
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = fm
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity = fm
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(s) = severity_filter {
            if severity != s {
                continue;
            }
        }
        if let Some(s) = status_filter {
            if status != s {
                continue;
            }
        }

        entries.push(FindingEntry {
            id,
            status,
            severity,
            filename: fname,
            path: Utf8PathBuf::from_path_buf(path).unwrap_or_default(),
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

/// Remove a finding by ID. Moves to OS trash. Caller handles confirmation.
pub fn remove_finding(
    ws: &Workspace,
    finding_id: &str,
) -> Result<crate::RemovalMethod, WorkspaceError> {
    let path = find_finding_file(ws, finding_id)?;
    crate::trash_or_delete(&path)
}

/// Gather all finding file paths under a given path.
/// Path can be a client, project, or engagement.
pub fn gather_findings(ws: &Workspace, path: &str) -> Result<Vec<Utf8PathBuf>, WorkspaceError> {
    let (root, _) = entities::resolve_existing_entity(ws, path)?;

    let mut findings = Vec::new();
    gather_findings_recursive(&root, &mut findings)?;
    findings.sort();
    Ok(findings)
}

fn gather_findings_recursive(
    dir: &Utf8PathBuf,
    findings: &mut Vec<Utf8PathBuf>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "findings" {
                for f in fs::read_dir(&path)? {
                    let f = f?;
                    let fp = f.path();
                    if fp.extension().and_then(|e| e.to_str()) == Some("md") {
                        findings.push(Utf8PathBuf::from_path_buf(fp).unwrap_or_default());
                    }
                }
            } else if path.join("config.toml").exists() {
                // Recurse into client/project/engagement directories
                gather_findings_recursive(
                    &Utf8PathBuf::from_path_buf(path).unwrap_or_default(),
                    findings,
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn create_and_find_finding() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let path = create_finding(&workspace, "acme/web_app/initial", "Stored XSS", true).unwrap();
        assert!(path.exists());

        // The ID should be ACME-WEB-001
        let found = find_finding_file(&workspace, "ACME-WEB-001").unwrap();
        assert_eq!(found, path);
    }

    #[test]
    fn finding_filename_format() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let path = create_finding(&workspace, "acme/web_app/initial", "Stored XSS", true).unwrap();
        let filename = path.file_name().unwrap();
        assert!(filename.starts_with("acme_web_001_"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn all_finding_status_changes_work() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "SQLi", true).unwrap();

        for status in [
            "fixed",
            "false_positive",
            "not_applicable",
            "risk_accepted",
            "open",
        ] {
            update_finding_status(&workspace, "ACME-WEB-001", status).unwrap();
            let content = show_finding(&workspace, "ACME-WEB-001").unwrap();
            assert!(content.contains(&format!("status: {}", status)));
        }
    }

    #[test]
    fn update_finding_severity_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "XSS", true).unwrap();

        update_finding_severity(&workspace, "ACME-WEB-001", "critical").unwrap();

        let content = show_finding(&workspace, "ACME-WEB-001").unwrap();
        assert!(content.contains("critical"));
    }

    #[test]
    fn list_findings_with_filters() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "Finding One", true).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "Finding Two", true).unwrap();

        update_finding_severity(&workspace, "ACME-WEB-001", "critical").unwrap();

        let eng_dir = ws.root.join("acme").join("web_app").join("initial");

        // No filter
        let all = list_findings(&eng_dir, None, None).unwrap();
        assert_eq!(all.len(), 2);

        // Filter by severity
        let critical = list_findings(&eng_dir, Some("critical"), None).unwrap();
        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].id, "ACME-WEB-001");

        // Filter by severity that matches none
        let none = list_findings(&eng_dir, Some("low"), None).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn remove_finding_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "Test", true).unwrap();
        assert!(find_finding_file(&workspace, "ACME-WEB-001").is_ok());

        remove_finding(&workspace, "ACME-WEB-001").unwrap();
        assert!(find_finding_file(&workspace, "ACME-WEB-001").is_err());
    }

    #[test]
    fn gather_findings_by_engagement() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");
        ws.create_engagement("acme", "web_app", "retest");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_finding(&workspace, "acme/web_app/initial", "F1", true).unwrap();
        create_finding(&workspace, "acme/web_app/retest", "F2", true).unwrap();

        let by_eng = gather_findings(&workspace, "acme/web_app/initial").unwrap();
        assert_eq!(by_eng.len(), 1);

        let by_project = gather_findings(&workspace, "acme/web_app").unwrap();
        assert_eq!(by_project.len(), 2);

        let by_client = gather_findings(&workspace, "acme").unwrap();
        assert_eq!(by_client.len(), 2);
    }
}
