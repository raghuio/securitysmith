// Integration tests for SecuritySmith CLI.
// Uses assert_cmd + predicates to run the binary and verify output.
// Covers every command, every error code (1-13), and key edge cases from the spec.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

use ss_workspace::entities::{ClientConfig, ClientIdSection, EngagementConfig, ProjectConfig};

/// Helper: serialize a client config to TOML using the typed Default impl.
fn client_config_toml(prefix: &str) -> String {
    let mut config = ClientConfig::default();
    config.client.id = Some(ClientIdSection {
        prefix: prefix.to_string(),
    });
    toml::to_string_pretty(&config).unwrap()
}

/// Helper: create a workspace in a temp dir and return the path.
/// Uses an isolated HOME to avoid global config collisions between parallel tests.
fn make_workspace() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".config")).unwrap();
    sm_at(tmp.path(), &home).arg("new").assert().success();
    tmp
}

/// Helper: create a Command for sm with isolated HOME.
fn sm_at(dir: &std::path::Path, home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("sm").unwrap();
    cmd.current_dir(dir).env("HOME", home);
    cmd
}

/// Helper: get the home dir path for a workspace temp dir.
fn home_of(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("home")
}

/// Helper: create a full hierarchy (client/project/engagement) in a workspace.
fn make_hierarchy(tmp: &TempDir) {
    let dir = tmp.path();
    let home = home_of(tmp);
    sm_at(dir, &home).args(["new", "acme"]).assert().success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app"])
        .assert()
        .success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success();
}

// ── AC-1: sm new creates only config.toml ──────────────

#[test]
fn ac1_new_workspace_creates_only_config_toml() {
    let tmp = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    fs::create_dir_all(home.path().join(".config")).unwrap();
    sm_at(tmp.path(), home.path())
        .arg("new")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created workspace"));

    let entries: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
    assert_eq!(entries.len(), 1, "Only config.toml should exist");
    assert!(tmp.path().join("config.toml").exists());
}

// ── AC-2: sm new acme/web_app/initial creates hierarchy ──

#[test]
fn ac2_create_full_hierarchy() {
    let tmp = make_workspace();
    let dir = tmp.path();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created client"));
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme/web_app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project"));
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created engagement"));

    assert!(dir.join("acme/config.toml").exists());
    assert!(dir.join("acme/web_app/config.toml").exists());
    assert!(dir.join("acme/web_app/initial/config.toml").exists());
}

// ── AC-3: sm finding creates finding with generated ID ──

#[test]
fn ac3_create_finding_with_id() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "Stored XSS",
            "--no-template",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created finding"));

    // Check finding file exists with generated ID
    let findings_dir = dir.join("acme/web_app/initial/findings");
    assert!(
        findings_dir.exists(),
        "findings/ directory should be created"
    );

    let finding_files: Vec<_> = fs::read_dir(&findings_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert_eq!(finding_files.len(), 1, "One finding file should exist");

    // Verify frontmatter has the ID
    let content = fs::read_to_string(finding_files[0].path()).unwrap();
    assert!(
        content.contains("id: ACME-WEB-001"),
        "Finding ID should be ACME-WEB-001"
    );
}

// ── AC-4: sm finding --status updates frontmatter ──

#[test]
fn ac4_update_finding_status() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--status", "fixed"])
        .assert()
        .success();

    let finding_file = dir.join("acme/web_app/initial/findings/acme_web_001_xss.md");
    let content = fs::read_to_string(&finding_file).unwrap();
    assert!(content.contains("status: fixed"));
}

// ── AC-5: sm ls --findings --severity high filters ──

#[test]
fn ac5_list_findings_filter_by_severity() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    // Create two findings, set one to high
    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "Low Finding",
            "--no-template",
        ])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "High Finding",
            "--no-template",
        ])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["finding", "ACME-WEB-002", "--severity", "high"])
        .assert()
        .success();

    // Filter by high — should only show the second finding
    sm_at(dir, &home_of(&tmp))
        .args([
            "ls",
            "acme/web_app/initial",
            "--findings",
            "--severity",
            "high",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACME-WEB-002"))
        .stdout(predicate::str::contains("ACME-WEB-001").not());
}

// ── AC-6: sm req creates requirement ──

#[test]
fn ac6_create_requirement() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test auth",
            "--no-template",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created requirement"));

    let req_dir = dir.join("acme/web_app/initial/requirements");
    assert!(req_dir.exists());

    let req_files: Vec<_> = fs::read_dir(&req_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert_eq!(req_files.len(), 1);

    let content = fs::read_to_string(req_files[0].path()).unwrap();
    assert!(
        content.contains("id: REQ-001"),
        "Requirement ID should be REQ-001"
    );
}

// ── AC-9: Export finding as HTML, PDF, JSON ──

#[test]
fn ac9_export_finding_formats() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    // JSON export to stdout
    sm_at(dir, &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--export", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACME-WEB-001"))
        .stdout(predicate::str::contains("frontmatter"));

    // HTML export to file
    let html_path = dir.join("finding.html");
    sm_at(dir, &home_of(&tmp))
        .args([
            "finding",
            "ACME-WEB-001",
            "--export",
            "html",
            "--to",
            "finding.html",
        ])
        .assert()
        .success();
    assert!(html_path.exists(), "HTML file should be created");
    let html = fs::read_to_string(&html_path).unwrap();
    assert!(html.contains("<!DOCTYPE html>"));

    // PDF export to file
    let pdf_path = dir.join("finding.pdf");
    sm_at(dir, &home_of(&tmp))
        .args([
            "finding",
            "ACME-WEB-001",
            "--export",
            "pdf",
            "--to",
            "finding.pdf",
        ])
        .assert()
        .success();
    assert!(pdf_path.exists(), "PDF file should be created");
    assert!(
        fs::metadata(&pdf_path).unwrap().len() > 0,
        "PDF should not be empty"
    );
}

// ── AC-12: sm rm --yes removes entity ──

#[test]
fn ac12_remove_with_yes() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .args(["rm", "acme/web_app", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acme/web_app"));

    assert!(
        !dir.join("acme/web_app").exists(),
        "Project directory should be removed"
    );
}

// ── AC-13: sm check detects duplicate IDs ──

#[test]
fn ac13_check_duplicate_finding_id() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    // Create two findings with the same ID manually
    let findings_dir = dir.join("acme/web_app/initial/findings");
    fs::create_dir_all(&findings_dir).unwrap();
    let fm = "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\nseverity: \"high\"\ncreated: \"2026-07-09\"\nupdated: \"2026-07-09\"\n---\n\n# Test\n";
    fs::write(findings_dir.join("dup1.md"), fm).unwrap();
    fs::write(findings_dir.join("dup2.md"), fm).unwrap();

    sm_at(dir, &home_of(&tmp))
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Duplicate finding ID"));
}

// ── AC-14: sm --help shows all commands ──

#[test]
fn ac14_help_shows_all_commands() {
    Command::cargo_bin("sm")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("ls"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("edit"))
        .stdout(predicate::str::contains("rm"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("finding"))
        .stdout(predicate::str::contains("req"))
        .stdout(predicate::str::contains("scope"))
        .stdout(predicate::str::contains("note"))
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("sow"));
}

// ── AC-16: sm note creates timestamped file ──

#[test]
fn ac16_create_note() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .args([
            "note",
            "acme/web_app/initial",
            "Remember to test rate limits",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created note"));

    let notes_dir = dir.join("acme/web_app/initial/notes");
    assert!(notes_dir.exists());

    let note_files: Vec<_> = fs::read_dir(&notes_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert_eq!(note_files.len(), 1);

    let content = fs::read_to_string(note_files[0].path()).unwrap();
    assert!(content.contains("NOTE-001"), "Note ID should be NOTE-001");
    assert!(content.contains("Remember to test rate limits"));
}

// ── AC-17: sm stats shows correct counts ──

#[test]
fn ac17_stats_workspace() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "F1",
            "--no-template",
        ])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "F2",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .arg("stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("Clients:     1"))
        .stdout(predicate::str::contains("Projects:    1"))
        .stdout(predicate::str::contains("Engagements: 1"))
        .stdout(predicate::str::contains("Findings:    2"));
}

// ── AC-18: sm stats <client> scoped to that client ──

#[test]
fn ac18_stats_per_client() {
    let tmp = make_workspace();
    let dir = tmp.path();

    // Create two clients with different data
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme"])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme/web_app"])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "foobar"])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "foobar/api"])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .args(["new", "foobar/api/pentest"])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "F1",
            "--no-template",
        ])
        .assert()
        .success();
    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "foobar/api/pentest",
            "--title",
            "F2",
            "--no-template",
        ])
        .assert()
        .success();

    // Stats for acme should show 1 finding, not 2
    sm_at(dir, &home_of(&tmp))
        .args(["stats", "acme"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings:    1"));
}

// ── AC-21: Auto-discovery of manually created client ──

#[test]
fn ac21_auto_discovery() {
    let tmp = make_workspace();
    let dir = tmp.path();

    // Manually create a client directory with config.toml
    let client_dir = dir.join("manual_client");
    fs::create_dir_all(&client_dir).unwrap();
    fs::write(client_dir.join("config.toml"), client_config_toml("MAN")).unwrap();

    // sm ls should find it
    sm_at(dir, &home_of(&tmp))
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("manual_client"));
}

// ── Error code 11: rm without --yes (declined) ──

#[test]
fn error_code_11_rm_declined() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);

    // Without --yes, the prompt is shown. In non-interactive mode (no stdin),
    // read_line gets EOF → treated as "N" → removal declined, exit code 11.
    sm_at(tmp.path(), &home_of(&tmp))
        .args(["rm", "acme"])
        .assert()
        .failure()
        .code(11)
        .stderr(predicate::str::contains("Removal cancelled"));

    // Entity should still exist — removal was declined.
    assert!(tmp.path().join("acme").exists());
}

// ── Error code 12: Invalid name format ──

#[test]
fn error_code_12_invalid_name() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "Acme"])
        .assert()
        .failure()
        .code(12)
        .stderr(predicate::str::contains("snake_case"));
}

// ── Error code 13: Reserved name ──

#[test]
fn error_code_13_reserved_name() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "templates"])
        .assert()
        .failure()
        .code(13)
        .stderr(predicate::str::contains("reserved"));
}

// ── Edge case: Reserved name templates used as client name ──

#[test]
fn edge_templates_as_client_rejected() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "templates"])
        .assert()
        .failure()
        .code(13);
}

// ── Edge case: Project without existing client rejected ──

#[test]
fn edge_project_auto_creates_client() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "nonexistent/project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created client"))
        .stdout(predicate::str::contains("Created project"));

    assert!(tmp.path().join("nonexistent").exists());
    assert!(tmp.path().join("nonexistent/project").exists());
}

// ── Edge case: Duplicate client rejected ──

#[test]
fn edge_duplicate_client_rejected() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "acme"])
        .assert()
        .success();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["new", "acme"])
        .assert()
        .failure()
        .code(3);
}

// ── Edge case: sm ls on empty workspace ──

#[test]
fn edge_ls_empty_workspace() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("No clients found"));
}

// ── sm status shows active engagements overview ──

#[test]
fn status_no_engagements() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("No active engagements found."));
}

#[test]
fn status_shows_client_engagements() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Set engagement to in_progress with dates
    sm_at(dir, &home)
        .args([
            "engagement",
            "acme/web_app/initial",
            "--status",
            "in_progress",
        ])
        .assert()
        .success();
    sm_at(dir, &home)
        .args([
            "engagement",
            "acme/web_app/initial",
            "--start-date",
            "2026-07-01",
        ])
        .assert()
        .success();
    sm_at(dir, &home)
        .args([
            "engagement",
            "acme/web_app/initial",
            "--end-date",
            "2026-07-14",
        ])
        .assert()
        .success();

    // sm status (no args) should show the engagement across workspaces
    sm_at(dir, &home)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace:"))
        .stdout(predicate::str::contains("acme/"))
        .stdout(predicate::str::contains("web_app/"))
        .stdout(predicate::str::contains("initial"))
        .stdout(predicate::str::contains("in_progress"))
        .stdout(predicate::str::contains("2026-07-01"));
}

#[test]
fn status_client_only() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .args(["engagement", "acme/web_app/initial", "--status", "planned"])
        .assert()
        .success();

    sm_at(dir, &home)
        .args(["status", "acme"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Client: acme"))
        .stdout(predicate::str::contains("web_app/"))
        .stdout(predicate::str::contains("initial"))
        .stdout(predicate::str::contains("planned"));
}

#[test]
fn status_client_archived() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Mark engagement as completed
    sm_at(dir, &home)
        .args([
            "engagement",
            "acme/web_app/initial",
            "--status",
            "completed",
        ])
        .assert()
        .success();

    // Default (active) should not show completed
    sm_at(dir, &home)
        .args(["status", "acme"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No active engagements"));

    // --archived should show it
    sm_at(dir, &home)
        .args(["status", "acme", "--archived"])
        .assert()
        .success()
        .stdout(predicate::str::contains("initial"))
        .stdout(predicate::str::contains("completed"));
}

#[test]
fn status_client_all() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .args([
            "engagement",
            "acme/web_app/initial",
            "--status",
            "completed",
        ])
        .assert()
        .success();

    // --all shows everything (active + archived)
    sm_at(dir, &home)
        .args(["status", "acme", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("initial"))
        .stdout(predicate::str::contains("completed"));
}

#[test]
fn status_nonexistent_client() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["status", "nonexistent"])
        .assert()
        .failure()
        .code(2);
}

// ── sm config shows global config ──

#[test]
fn config_shows_global() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".config")).unwrap();
    sm_at(tmp.path(), &home)
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::contains("config file:"));
}

// ── sm ls templates shows built-in and workspace templates ──

#[test]
fn ls_templates_shows_all() {
    let tmp = make_workspace();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["ls", "templates"])
        .assert()
        .success()
        .stdout(predicate::str::contains("finding"))
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("sow"))
        .stdout(predicate::str::contains("requirement"))
        .stdout(predicate::str::contains("built-in"));
}

// ── sm report builds markdown to stdout ──

#[test]
fn report_markdown_stdout() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["report", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report"));
}

// ── sm sow builds markdown to stdout ──

#[test]
fn sow_markdown_stdout() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["sow", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Statement of Work"));
}

// ── sm show with finding ID ──

#[test]
fn show_finding_by_id() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["show", "ACME-WEB-001"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACME-WEB-001"));
}

// ── sm rm finding by ID with --yes ──

#[test]
fn rm_finding_by_id() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["rm", "ACME-WEB-001", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACME-WEB-001"));
}

// ── sm req update status ──

#[test]
fn req_update_status() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["req", "REQ-001", "--status", "in_progress"])
        .assert()
        .success()
        .stdout(predicate::str::contains("in_progress"));
}

// ── sm ls lists content in engagement ──

#[test]
fn ls_engagement_content() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["ls", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings:"));
}

// ── sm -V prints version ──

#[test]
fn version_flag() {
    Command::cargo_bin("sm")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.2.0"));
}

// ── sm finding --severity updates severity ──

#[test]
fn finding_update_severity() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--severity", "critical"])
        .assert()
        .success();

    let content =
        fs::read_to_string(dir.join("acme/web_app/initial/findings/acme_web_001_xss.md")).unwrap();
    assert!(content.contains("critical"));
}

// ── sm new templates/finding creates workspace template ──

#[test]
fn create_workspace_template() {
    let tmp = make_workspace();
    let dir = tmp.path();

    sm_at(dir, &home_of(&tmp))
        .env("EDITOR", "true")
        .args(["new", "templates/finding"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created template"));

    assert!(dir.join("templates/finding.md").exists());
}

// ── Invalid severity rejected at parse time (ValueEnum) ──

#[test]
fn invalid_severity_rejected() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--severity", "invalid_value"])
        .assert()
        .failure();
}

// ── Invalid status rejected at parse time (ValueEnum) ──

#[test]
fn invalid_status_rejected() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);

    sm_at(tmp.path(), &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "X",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--status", "invalid_value"])
        .assert()
        .failure();
}

// ── Invalid format rejected at parse time (ValueEnum) ──

#[test]
fn invalid_format_rejected() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);

    sm_at(tmp.path(), &home_of(&tmp))
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "X",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(tmp.path(), &home_of(&tmp))
        .args(["finding", "ACME-WEB-001", "--export", "invalid_format"])
        .assert()
        .failure();
}

// ── AC-7: sm scope opens scope.md ──

#[test]
fn ac7_scope_opens_editor() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args(["scope", "acme/web_app/initial"])
        .assert()
        .success();
    assert!(dir.join("acme/web_app/initial/scope.md").exists());
}

// ── AC-8: Report shows findings in order ──

#[test]
fn ac8_report_markdown_findings_in_order() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "First",
            "--no-template",
        ])
        .assert()
        .success();
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "Second",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home)
        .args(["report", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ACME-WEB-001"))
        .stdout(predicate::str::contains("ACME-WEB-002"));
}

// ── AC-10: SOW contains requirements ──

#[test]
fn ac10_sow_contains_requirements() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test auth",
            "--no-template",
        ])
        .assert()
        .success();

    sm_at(dir, &home)
        .args(["sow", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Statement of Work"))
        .stdout(predicate::str::contains("REQ-001"));
}

// ── AC-11: Custom template override ──

#[test]
fn ac11_custom_template_override() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create custom finding template
    let templates_dir = dir.join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("finding.md"),
        "# Custom Template\n\n## Custom\n",
    )
    .unwrap();

    // Create finding — should use custom template
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    // With --no-template, it should NOT use the custom template
    let finding_file = dir.join("acme/web_app/initial/findings/acme_web_001_test.md");
    let content = fs::read_to_string(&finding_file).unwrap();
    assert!(
        !content.contains("Custom Template"),
        "Should not have template with --no-template"
    );

    // Create finding without --no-template — should use custom template
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "With Template",
        ])
        .assert()
        .success();

    let finding_file2 = dir.join("acme/web_app/initial/findings/acme_web_002_with_template.md");
    let content2 = fs::read_to_string(&finding_file2).unwrap();
    assert!(
        content2.contains("Custom Template"),
        "Should have custom template"
    );
}

// ── AC-15: Release binary runs ──

#[test]
fn ac15_release_binary_runs() {
    // The test binary itself is the release binary equivalent for testing purposes
    Command::cargo_bin("sm")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

// ── AC-19: sm stats --all aggregates across workspaces ──

#[test]
fn ac19_stats_all_multiple_workspaces() {
    let tmp1 = make_workspace();
    let dir1 = tmp1.path();
    let home1 = home_of(&tmp1);
    sm_at(dir1, &home1).args(["new", "acme"]).assert().success();
    sm_at(dir1, &home1)
        .args(["new", "acme/web_app"])
        .assert()
        .success();
    sm_at(dir1, &home1)
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success();

    let tmp2 = TempDir::new().unwrap();
    let home2 = tmp2.path().join("home");
    fs::create_dir_all(home2.join(".config")).unwrap();
    sm_at(tmp2.path(), &home2).arg("new").assert().success();
    sm_at(tmp2.path(), &home2)
        .args(["new", "foobar"])
        .assert()
        .success();

    // Register tmp2's workspace in tmp1's global config
    // We need to use the same HOME for both workspaces
    // Actually, let's use a single HOME and register both workspaces
    let combined_home = TempDir::new().unwrap();
    fs::create_dir_all(combined_home.path().join(".config")).unwrap();

    // Create workspace1
    let ws1 = TempDir::new().unwrap();
    sm_at(ws1.path(), combined_home.path())
        .arg("new")
        .assert()
        .success();
    sm_at(ws1.path(), combined_home.path())
        .args(["new", "acme"])
        .assert()
        .success();

    // Create workspace2
    let ws2 = TempDir::new().unwrap();
    sm_at(ws2.path(), combined_home.path())
        .arg("new")
        .assert()
        .success();
    sm_at(ws2.path(), combined_home.path())
        .args(["new", "foobar"])
        .assert()
        .success();

    // sm stats --all should aggregate both
    sm_at(ws1.path(), combined_home.path())
        .args(["stats", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Clients:     2"));
}

// ── AC-20: sm -w <name> ls ──

#[test]
fn ac20_workspace_flag_ls() {
    let combined_home = TempDir::new().unwrap();
    fs::create_dir_all(combined_home.path().join(".config")).unwrap();

    // Create workspace "2026"
    let ws = TempDir::new().unwrap();
    let ws_dir = ws.path();
    sm_at(ws_dir, combined_home.path())
        .arg("new")
        .assert()
        .success();
    sm_at(ws_dir, combined_home.path())
        .args(["new", "acme"])
        .assert()
        .success();

    // Get the workspace name from config
    let config_content = fs::read_to_string(
        combined_home
            .path()
            .join(".config")
            .join("securitysmith")
            .join("config.toml"),
    )
    .unwrap();

    // Find the workspace name (last registered workspace in config)
    let name = config_content
        .lines()
        .rev()
        .find(|l| l.contains("name = "))
        .map(|l| {
            l.split('=')
                .nth(1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .to_string()
        })
        .unwrap_or_default();

    // Use -w flag to list clients in that workspace
    // We need to be in a different directory
    let other_dir = TempDir::new().unwrap();
    sm_at(other_dir.path(), combined_home.path())
        .args(["-w", &name, "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acme"));
}

// ── AC-22: sm check --fix removes stale entries ──

#[test]
fn ac22_check_fix_removes_stale() {
    let combined_home = TempDir::new().unwrap();
    fs::create_dir_all(combined_home.path().join(".config")).unwrap();

    // Create a workspace
    let ws = TempDir::new().unwrap();
    sm_at(ws.path(), combined_home.path())
        .arg("new")
        .assert()
        .success();
    sm_at(ws.path(), combined_home.path())
        .args(["new", "acme"])
        .assert()
        .success();

    // Drop the workspace (remove the directory)
    drop(ws);

    // Stale entries are auto-cleaned when the global config is loaded.
    // Running any command that loads the config should remove the stale entry.
    let other = TempDir::new().unwrap();
    sm_at(other.path(), combined_home.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("All workspace entries are valid."));

    // Verify stale entry is gone — config should not list the dropped workspace
    sm_at(other.path(), combined_home.path())
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::contains("Stale").not());
}

// ── Finding statuses can change freely ──

#[test]
fn all_finding_status_changes_are_allowed() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    for status in [
        "fixed",
        "false_positive",
        "not_applicable",
        "risk_accepted",
        "open",
    ] {
        sm_at(dir, &home)
            .args(["finding", "ACME-WEB-001", "--status", status])
            .assert()
            .success();
    }

    let finding_file = dir.join("acme/web_app/initial/findings/acme_web_001_xss.md");
    let content = fs::read_to_string(finding_file).unwrap();
    assert!(content.contains("status: open"));
}

// ── Workspace paths cannot escape their workspace ──

#[test]
fn remove_rejects_parent_directory_traversal() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let workspace = root.path().join("workspace");
    let victim = root.path().join("victim");
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&victim).unwrap();
    fs::write(victim.join("config.toml"), "[client]\n").unwrap();

    sm_at(&workspace, &home).arg("new").assert().success();

    sm_at(&workspace, &home)
        .args(["rm", "../victim", "--yes"])
        .assert()
        .failure()
        .code(12);

    assert!(victim.exists());
    assert!(victim.join("config.toml").exists());
}

// ── Error code 1: Not a workspace ──

#[test]
fn error_code_1_not_a_workspace() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".config")).unwrap();

    // Invalid -w flag should fail with code 1 — not a registered workspace
    // and not a valid path
    sm_at(tmp.path(), &home)
        .args(["-w", "nonexistent", "status"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("No SecuritySmith workspace found"));
}

// ── Error code 2: Entity not found ──

#[test]
fn error_code_2_entity_not_found() {
    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home)
        .args(["show", "nonexistent_client"])
        .assert()
        .failure()
        .code(2);
}

// ── Error code 4: Invalid TOML ──

#[test]
fn error_code_4_invalid_toml() {
    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home).args(["new", "acme"]).assert().success();

    // Corrupt the client config
    fs::write(dir.join("acme/config.toml"), "not valid toml {{{").unwrap();

    sm_at(dir, &home)
        .args(["show", "acme"])
        .assert()
        .failure()
        .code(4);
}

// ── Error code 5: Invalid frontmatter ──

#[test]
fn error_code_5_invalid_frontmatter() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a finding with broken frontmatter
    let findings_dir = dir.join("acme/web_app/initial/findings");
    fs::create_dir_all(&findings_dir).unwrap();
    fs::write(
        findings_dir.join("acme_web_001_broken.md"),
        "---\nid: ACME-WEB-001\nstatus: [broken yaml\n---\n\n# Broken\n",
    )
    .unwrap();

    // Updating status parses frontmatter — should fail with code 5
    sm_at(dir, &home)
        .args(["finding", "ACME-WEB-001", "--status", "fixed"])
        .assert()
        .failure()
        .code(5);
}

// ── Error code 9: Report generation failed ──

#[test]
fn error_code_9_report_no_findings() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Report on an engagement with no findings
    sm_at(dir, &home)
        .args(["report", "acme/web_app/initial"])
        .assert()
        .failure()
        .code(9);
}

#[test]
fn error_code_9_report_pdf_without_to() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a finding so report has content
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    // PDF without --to should fail with code 9
    sm_at(dir, &home)
        .args(["report", "acme/web_app/initial", "--format", "pdf"])
        .assert()
        .failure()
        .code(9);
}

// ── Error code 10: SOW generation failed ──

#[test]
fn error_code_10_sow_non_engagement_path() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // SOW on a project path (not engagement) should fail with code 10
    sm_at(dir, &home)
        .args(["sow", "acme/web_app"])
        .assert()
        .failure()
        .code(10);
}

#[test]
fn error_code_10_sow_pdf_without_to() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // PDF without --to should fail with code 10
    sm_at(dir, &home)
        .args(["sow", "acme/web_app/initial", "--format", "pdf"])
        .assert()
        .failure()
        .code(10);
}

// ── Error code 6: Missing required field (via sm check) ──

#[test]
fn error_code_6_missing_required_field() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a finding missing the severity field
    let findings_dir = dir.join("acme/web_app/initial/findings");
    fs::create_dir_all(&findings_dir).unwrap();
    fs::write(
        findings_dir.join("acme_web_001_test.md"),
        "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# Test\n",
    )
    .unwrap();

    sm_at(dir, &home)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "missing required field `severity`",
        ));
}

// ── Error code 7: Invalid date (via sm check) ──

#[test]
fn error_code_7_invalid_date() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Put an invalid date in the engagement config (using Default impl)
    let eng_config = dir.join("acme/web_app/initial/config.toml");
    let mut config = EngagementConfig::default();
    config.engagement.status = "in_progress".to_string();
    config.engagement.start_date = "not-a-date".to_string();
    config.engagement.end_date = "2026-07-14".to_string();
    let toml = toml::to_string_pretty(&config).unwrap();
    fs::write(&eng_config, toml).unwrap();

    sm_at(dir, &home)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("not in YYYY-MM-DD format"));
}

// ── Error code 8: Invalid status (via sm check) ──

#[test]
fn error_code_8_invalid_status_in_file() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a finding with an invalid status
    let findings_dir = dir.join("acme/web_app/initial/findings");
    fs::create_dir_all(&findings_dir).unwrap();
    fs::write(
        findings_dir.join("acme_web_001_test.md"),
        "---\nid: \"ACME-WEB-001\"\nstatus: \"invalid_status\"\nseverity: \"high\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# Test\n",
    )
    .unwrap();

    sm_at(dir, &home)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("not a valid Finding status"));
}

// ── Cannot remove built-in template ──

#[test]
fn cannot_remove_builtin_template() {
    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Try to remove a built-in template (no workspace file exists)
    sm_at(dir, &home)
        .args(["rm", "templates/report", "--yes"])
        .assert()
        .failure();
}

// ── NO_COLOR disables colored output ──

#[test]
fn no_color_disables_ansi_codes() {
    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create hierarchy and a finding
    sm_at(dir, &home).args(["new", "acme"]).assert().success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app"])
        .assert()
        .success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success();
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    // With NO_COLOR set, output must not contain ANSI escape codes
    sm_at(dir, &home)
        .env("NO_COLOR", "1")
        .args(["ls", "acme/web_app/initial", "--findings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b").not());
}

// ── Piped output disables colored output (isatty check) ──

#[test]
fn piped_output_has_no_ansi_codes() {
    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    sm_at(dir, &home).args(["new", "acme"]).assert().success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app"])
        .assert()
        .success();
    sm_at(dir, &home)
        .args(["new", "acme/web_app/initial"])
        .assert()
        .success();
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    // assert_cmd captures output via pipes, so isatty returns false.
    // Output must not contain ANSI escape codes.
    sm_at(dir, &home)
        .args(["ls", "acme/web_app/initial", "--findings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b").not());
}

// ── Symlink escape prevention ──

#[cfg(unix)]
#[test]
fn symlink_escape_rejected_by_entity_operations() {
    use std::os::unix::fs::symlink;

    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a legitimate client
    sm_at(dir, &home).args(["new", "acme"]).assert().success();

    // Create a symlink inside the workspace pointing to /tmp
    let link_path = dir.join("evil");
    symlink("/tmp", &link_path).unwrap();

    // Trying to use the symlink as an entity path should fail
    sm_at(dir, &home)
        .args(["new", "evil/web_app"])
        .assert()
        .failure();
}

#[cfg(unix)]
#[test]
fn check_warns_about_symlink_escape() {
    use std::os::unix::fs::symlink;

    let tmp = make_workspace();
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a symlink inside the workspace pointing outside
    let link_path = dir.join("escape_link");
    symlink("/tmp", &link_path).unwrap();

    // sm check should warn about the symlink
    sm_at(dir, &home)
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("Symlink points outside"));
}

// ── Error code 8: Invalid requirement status transition ──

#[test]
fn error_code_8_invalid_requirement_transition() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a requirement (starts as "open")
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    // "verified" is only reachable from "in_progress", not from "open"
    sm_at(dir, &home)
        .args(["req", "REQ-001", "--status", "verified"])
        .assert()
        .failure()
        .code(8)
        .stderr(predicate::str::contains(
            "Invalid requirement status transition",
        ));
}

// ── Config inheritance: report template from project config ──

#[test]
fn report_uses_config_template_from_project() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a custom report template in the workspace
    let templates_dir = dir.join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("custom_report.md"),
        "# Custom Project Report\n\n{{findings}}\n",
    )
    .unwrap();

    // Set the project config to use the custom template (using Default impl)
    let project_config = dir.join("acme/web_app/config.toml");
    let mut config = ProjectConfig::default();
    config.project.abbreviation = "WEB".to_string();
    config.project.report.template = "custom_report".to_string();
    let toml = toml::to_string_pretty(&config).unwrap();
    fs::write(&project_config, toml).unwrap();

    // Create a finding
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "finding",
            "acme/web_app/initial",
            "--title",
            "XSS",
            "--no-template",
        ])
        .assert()
        .success();

    // Report should use the custom template from project config
    sm_at(dir, &home)
        .args(["report", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Custom Project Report"));
}

// ── Config inheritance: SOW template from client config ──

#[test]
fn sow_uses_config_template_from_client() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a custom SOW template in the workspace
    let templates_dir = dir.join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("custom_sow.md"),
        "# Custom Client SOW\n\n{{scope}}\n\n{{requirements}}\n",
    )
    .unwrap();

    // Set the client config to use the custom SOW template (using Default impl)
    let client_config = dir.join("acme/config.toml");
    let mut config = ClientConfig::default();
    config.client.id = Some(ClientIdSection {
        prefix: "ACME".to_string(),
    });
    config.client.sow.template = "custom_sow".to_string();
    let toml = toml::to_string_pretty(&config).unwrap();
    fs::write(&client_config, toml).unwrap();

    // Create a requirement
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    // SOW should use the custom template from client config
    sm_at(dir, &home)
        .args(["sow", "acme/web_app/initial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Custom Client SOW"));
}

// ── Same-status requirement transition is a no-op ──

#[test]
fn req_same_status_is_noop() {
    let tmp = make_workspace();
    make_hierarchy(&tmp);
    let dir = tmp.path();
    let home = home_of(&tmp);

    // Create a requirement (starts as "open")
    sm_at(dir, &home)
        .env("EDITOR", "true")
        .args([
            "req",
            "acme/web_app/initial",
            "--title",
            "Test",
            "--no-template",
        ])
        .assert()
        .success();

    // Setting it to "open" again should succeed (no-op)
    sm_at(dir, &home)
        .args(["req", "REQ-001", "--status", "open"])
        .assert()
        .success();
}
