//! Health checks — schema validation, duplicate IDs, orphaned files, stale config.
//!
//! `sm check` reports issues. `sm check --fix` removes stale workspace entries.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::Workspace;
use crate::global::GlobalConfig;

/// A health check issue.
#[derive(Debug, Clone)]
pub struct CheckIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

/// Run health checks on a workspace.
pub fn check_workspace(ws: &Workspace) -> Vec<CheckIssue> {
    let mut issues = Vec::new();

    check_duplicate_finding_ids(ws, &mut issues);
    check_finding_frontmatter(ws, &mut issues);
    check_requirement_frontmatter(ws, &mut issues);
    check_finding_values(ws, &mut issues);
    check_requirement_values(ws, &mut issues);
    check_config_dates(ws, &mut issues);
    check_engagement_statuses(ws, &mut issues);
    check_evidence_hashes(ws, &mut issues);
    check_evidence_secrets(ws, &mut issues);
    check_credential_store(ws, &mut issues);
    check_orphaned_dirs(ws, &mut issues);
    check_symlinks(ws, &mut issues);

    issues
}

/// Check for stale workspace entries in global config.
pub fn check_stale_workspaces(global: &GlobalConfig) -> Vec<CheckIssue> {
    let mut issues = Vec::new();
    for ws in &global.workspaces {
        if !ws.path.as_std_path().exists() {
            issues.push(CheckIssue {
                severity: IssueSeverity::Error,
                message: format!(
                    "Stale workspace entry: {} at {} (path no longer exists)",
                    ws.name, ws.path
                ),
                path: Some(ws.path.to_string()),
            });
        }
    }
    issues
}

// ── Shared workspace walker ──────────────────────────────────────────
//
// All check functions that inspect engagement content share the same
// 3-level traversal: client → project → engagement. This helper eliminates
// the duplication. The callback receives the engagement directory path.

fn for_each_engagement<F: FnMut(&Path)>(ws: &Workspace, mut f: F) {
    for client_entry in fs::read_dir(&ws.root).into_iter().flatten().flatten() {
        if !client_entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let client_dir = client_entry.path();
        if !client_dir.join("config.toml").exists() {
            continue;
        }
        for project_entry in fs::read_dir(&client_dir).into_iter().flatten().flatten() {
            if !project_entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(false)
            {
                continue;
            }
            let project_dir = project_entry.path();
            if !project_dir.join("config.toml").exists() {
                continue;
            }
            for eng_entry in fs::read_dir(&project_dir).into_iter().flatten().flatten() {
                if !eng_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let eng_dir = eng_entry.path();
                if !eng_dir.join("config.toml").exists() {
                    continue;
                }
                f(&eng_dir);
            }
        }
    }
}

/// Iterate over all `.md` files in `<engagement>/<subdir>/`, calling f for each.
fn for_each_md_in_subdir<F: FnMut(&Path)>(engagement_dir: &Path, subdir: &str, mut f: F) {
    let content_dir = engagement_dir.join(subdir);
    if !content_dir.exists() {
        return;
    }
    for f_entry in fs::read_dir(&content_dir).into_iter().flatten().flatten() {
        let f_path = f_entry.path();
        if f_path.extension().and_then(|e| e.to_str()) == Some("md") {
            f(&f_path);
        }
    }
}

// ── Individual checks ────────────────────────────────────────────────

/// Find duplicate finding IDs in the workspace.
fn check_duplicate_finding_ids(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    let mut seen: HashSet<String> = HashSet::new();

    for_each_engagement(ws, |eng_dir| {
        for_each_md_in_subdir(eng_dir, "findings", |f_path| {
            if let Ok(parsed) = ss_frontmatter::parse_file(f_path) {
                if let Some(id) = parsed.frontmatter.get("id").and_then(|v| v.as_str()) {
                    if !seen.insert(id.to_string()) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!("Duplicate finding ID: {}", id),
                            path: path_to_string(f_path),
                        });
                    }
                }
            }
        });
    });
}

/// Check that finding frontmatter has required fields.
fn check_finding_frontmatter(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    check_frontmatter_fields(
        ws,
        "findings",
        crate::entities::FINDING_REQUIRED_FIELDS,
        "Finding",
        issues,
    );
}

/// Check that requirement frontmatter has required fields.
fn check_requirement_frontmatter(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    check_frontmatter_fields(
        ws,
        "requirements",
        crate::entities::REQUIREMENT_REQUIRED_FIELDS,
        "Requirement",
        issues,
    );
}

fn check_frontmatter_fields(
    ws: &Workspace,
    subdir: &str,
    required: &[&str],
    entity_label: &str,
    issues: &mut Vec<CheckIssue>,
) {
    for_each_engagement(ws, |eng_dir| {
        for_each_md_in_subdir(eng_dir, subdir, |f_path| {
            if let Ok(parsed) = ss_frontmatter::parse_file(f_path) {
                let id = parsed
                    .frontmatter
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                for field in required {
                    if parsed.frontmatter.get(*field).is_none()
                        || parsed.frontmatter.get(*field) == Some(&serde_yaml::Value::Null)
                    {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!(
                                "{} `{}` is missing required field `{}`.",
                                entity_label, id, field
                            ),
                            path: path_to_string(f_path),
                        });
                    }
                }
            }
        });
    });
}

/// Check finding frontmatter for invalid status and severity values.
fn check_finding_values(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        for_each_md_in_subdir(eng_dir, "findings", |f_path| {
            if let Ok(parsed) = ss_frontmatter::parse_file(f_path) {
                let path_str = path_to_string(f_path);
                if let Some(val) = parsed.frontmatter.get("status").and_then(|v| v.as_str()) {
                    if !crate::entities::is_valid_value(
                        val,
                        crate::entities::VALID_FINDING_STATUSES,
                    ) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!("`{}` is not a valid Finding status.", val),
                            path: path_str.clone(),
                        });
                    }
                }
                if let Some(val) = parsed.frontmatter.get("severity").and_then(|v| v.as_str()) {
                    if !crate::entities::is_valid_value(val, crate::entities::VALID_SEVERITIES) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!("`{}` is not a valid Finding severity.", val),
                            path: path_str,
                        });
                    }
                }
            }
        });
    });
}

/// Check requirement frontmatter for invalid status values.
fn check_requirement_values(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        for_each_md_in_subdir(eng_dir, "requirements", |f_path| {
            if let Ok(parsed) = ss_frontmatter::parse_file(f_path) {
                if let Some(val) = parsed.frontmatter.get("status").and_then(|v| v.as_str()) {
                    if !crate::entities::is_valid_value(
                        val,
                        crate::entities::VALID_REQUIREMENT_STATUSES,
                    ) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!("`{}` is not a valid Requirement status.", val),
                            path: path_to_string(f_path),
                        });
                    }
                }
            }
        });
    });
}

/// Check for invalid dates in engagement configs.
fn check_config_dates(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        let config_path = eng_dir.join("config.toml");
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<crate::entities::EngagementConfig>(&content) {
                for (_label, date) in [
                    ("start_date", &config.engagement.start_date),
                    ("end_date", &config.engagement.end_date),
                ] {
                    if !date.is_empty() && !crate::entities::is_valid_date(date) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Error,
                            message: format!("Date `{}` is not in YYYY-MM-DD format.", date),
                            path: path_to_string(&config_path),
                        });
                    }
                }
            }
        }
    });
}

/// Check for directories that look like entities but have no config.toml.
fn check_orphaned_dirs(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for client_entry in fs::read_dir(&ws.root).into_iter().flatten().flatten() {
        if !client_entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let client_dir = client_entry.path();
        let name = client_entry.file_name().to_string_lossy().to_string();
        if name == "templates" {
            continue;
        }
        if !client_dir.join("config.toml").exists() {
            let has_project = fs::read_dir(&client_dir)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .any(|e| {
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && e.path().join("config.toml").exists()
                });
            if has_project {
                issues.push(CheckIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Client directory `{}` has no config.toml", name),
                    path: path_to_string(&client_dir),
                });
            }
        }
    }
}

/// Check for symlinks inside the workspace that point outside it.
fn check_symlinks(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    let canonical_root = match fs::canonicalize(ws.root.as_std_path()) {
        Ok(p) => p,
        Err(_) => return,
    };

    fn walk_for_symlinks(
        dir: &Path,
        canonical_root: &Path,
        issues: &mut Vec<CheckIssue>,
        depth: usize,
    ) {
        if depth > 5 {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                if let Ok(target) = fs::canonicalize(&path) {
                    if !target.starts_with(canonical_root) {
                        issues.push(CheckIssue {
                            severity: IssueSeverity::Warning,
                            message: "Symlink points outside the workspace".to_string(),
                            path: Some(path.to_string_lossy().to_string()),
                        });
                    }
                }
            } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                walk_for_symlinks(&path, canonical_root, issues, depth + 1);
            }
        }
    }

    walk_for_symlinks(ws.root.as_std_path(), &canonical_root, issues, 0);
}

/// Check engagement config.toml for invalid status values.
/// Spec: engagements/management FR-E8.
fn check_engagement_statuses(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        let config_path = eng_dir.join("config.toml");
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<crate::entities::EngagementConfig>(&content) {
                let status = &config.engagement.status;
                if !crate::entities::is_valid_engagement_status(status) {
                    issues.push(CheckIssue {
                        severity: IssueSeverity::Error,
                        message: format!(
                            "`{}` is not a valid engagement status. Use: {}.",
                            status,
                            crate::entities::VALID_ENGAGEMENT_STATUSES.join(", ")
                        ),
                        path: path_to_string(&config_path),
                    });
                }
            }
        }
    });
}

/// Check evidence file hashes against the evidence index.
/// Spec: findings/evidence FR-7.
fn check_evidence_hashes(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        if let Some(eng_utf8) = camino::Utf8Path::from_path(eng_dir) {
            if let Ok(mismatches) = crate::evidence::verify_hashes(eng_utf8) {
                for m in &mismatches {
                    issues.push(CheckIssue {
                        severity: IssueSeverity::Warning,
                        message: format!("Evidence {}", m),
                        path: path_to_string(eng_dir),
                    });
                }
            }
        }
    });
}

/// Scan text evidence files for common secret patterns.
/// Spec: findings/evidence FR-8.
fn check_evidence_secrets(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    for_each_engagement(ws, |eng_dir| {
        if let Some(eng_utf8) = camino::Utf8Path::from_path(eng_dir) {
            if let Ok(warnings) = crate::evidence::scan_for_secrets(eng_utf8) {
                for w in &warnings {
                    issues.push(CheckIssue {
                        severity: IssueSeverity::Warning,
                        message: format!("Potential secret in evidence: {}", w),
                        path: path_to_string(eng_dir),
                    });
                }
            }
        }
    });
}

/// Check that the encrypted credential store file is intact.
/// Verifies file exists and has the correct magic header.
/// Full decryption requires a password, which is not part of `sm check`.
/// Spec: credentials/management FR-12.
fn check_credential_store(ws: &Workspace, issues: &mut Vec<CheckIssue>) {
    let cred_path = ws.root.join(".credentials.enc");
    if !cred_path.exists() {
        return; // No credential store — nothing to check
    }
    if let Ok(bytes) = fs::read(cred_path.as_std_path()) {
        if bytes.len() < 7 || &bytes[..7] != b"SSCRED\x01" {
            issues.push(CheckIssue {
                severity: IssueSeverity::Error,
                message: "Credential store has invalid format (bad magic header).".to_string(),
                path: Some(cred_path.to_string()),
            });
        }
    } else {
        issues.push(CheckIssue {
            severity: IssueSeverity::Error,
            message: "Cannot read credential store file.".to_string(),
            path: Some(cred_path.to_string()),
        });
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Convert a Path to an optional String for CheckIssue.path.
fn path_to_string(path: &Path) -> Option<String> {
    camino::Utf8PathBuf::from_path_buf(path.to_path_buf())
        .ok()
        .map(|p| p.to_string())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn check_clean_workspace() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
    }

    #[test]
    fn check_duplicate_finding_id() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let findings_dir = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("findings");
        fs::create_dir_all(&findings_dir).unwrap();
        fs::write(
            findings_dir.join("acme_web_001_a.md"),
            "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\nseverity: \"high\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# A\n",
        )
        .unwrap();
        fs::write(
            findings_dir.join("acme_web_001_b.md"),
            "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\nseverity: \"high\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# B\n",
        )
        .unwrap();

        let issues = check_workspace(&workspace);
        let has_dup = issues
            .iter()
            .any(|i| i.message.contains("Duplicate finding ID"));
        assert!(has_dup, "Expected duplicate ID issue, got: {:?}", issues);
    }

    #[test]
    fn check_missing_frontmatter_field() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let findings_dir = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("findings");
        fs::create_dir_all(&findings_dir).unwrap();
        fs::write(
            findings_dir.join("acme_web_001_test.md"),
            "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# Test\n",
        )
        .unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        let has_missing = issues
            .iter()
            .any(|i| i.message.contains("missing required field `severity`"));
        assert!(
            has_missing,
            "Expected missing field issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_invalid_finding_status() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let findings_dir = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("findings");
        fs::create_dir_all(&findings_dir).unwrap();
        fs::write(
            findings_dir.join("acme_web_001_test.md"),
            "---\nid: \"ACME-WEB-001\"\nstatus: \"invalid_status\"\nseverity: \"high\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# Test\n",
        )
        .unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("not a valid Finding status")),
            "Expected invalid status issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_invalid_finding_severity() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let findings_dir = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("findings");
        fs::create_dir_all(&findings_dir).unwrap();
        fs::write(
            findings_dir.join("acme_web_001_test.md"),
            "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\nseverity: \"urgent\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# Test\n",
        )
        .unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("not a valid Finding severity")),
            "Expected invalid severity issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_invalid_date_in_config() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let eng_config = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("config.toml");
        let mut config = crate::entities::EngagementConfig::default();
        config.engagement.status = "in_progress".to_string();
        config.engagement.start_date = "not-a-date".to_string();
        config.engagement.end_date = "2026-07-14".to_string();
        let toml = toml::to_string_pretty(&config).unwrap();
        fs::write(&eng_config, toml).unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("not in YYYY-MM-DD format")),
            "Expected invalid date issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_invalid_engagement_status() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let eng_config = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("config.toml");
        let mut config = crate::entities::EngagementConfig::default();
        config.engagement.status = "invalid_status".to_string();
        let toml = toml::to_string_pretty(&config).unwrap();
        fs::write(&eng_config, toml).unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("not a valid engagement status")),
            "Expected invalid engagement status issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_evidence_hash_mismatch_detected() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        let eng = ws.create_engagement("acme", "web_app", "initial");

        // Add evidence
        let src = ws.root.join("test.txt");
        fs::write(&src, "original").unwrap();
        crate::evidence::add_evidence(&ws.root, &eng, &src).unwrap();

        // Modify the evidence file to cause a hash mismatch
        let evidence_dir = eng.join("evidence");
        let files: Vec<_> = fs::read_dir(&evidence_dir).unwrap().flatten().collect();
        if !files.is_empty() {
            fs::write(files[0].path(), "modified content").unwrap();
        }

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues.iter().any(|i| i.message.contains("Hash mismatch")),
            "Expected hash mismatch issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_evidence_secret_detected() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        let eng = ws.create_engagement("acme", "web_app", "initial");

        let src = ws.root.join("config.txt");
        fs::write(&src, "password=secret123").unwrap();
        crate::evidence::add_evidence(&ws.root, &eng, &src).unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("Potential secret")),
            "Expected secret warning, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_credential_store_bad_magic() {
        let ws = TestWorkspace::new();
        // Write a fake credential file with wrong magic
        fs::write(ws.root.join(".credentials.enc"), b"BADFORMAT\x00extra data").unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            issues.iter().any(|i| i.message.contains("invalid format")),
            "Expected credential store format issue, got: {:?}",
            issues
        );
    }

    #[test]
    fn check_credential_store_valid_magic_no_issue() {
        let ws = TestWorkspace::new();
        // Write a credential file with correct magic header
        fs::write(
            ws.root.join(".credentials.enc"),
            b"SSCRED\x01rest of encrypted data",
        )
        .unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let issues = check_workspace(&workspace);
        assert!(
            !issues.iter().any(|i| i.message.contains("credential")),
            "Should not report credential issue for valid magic, got: {:?}",
            issues
        );
    }
}
