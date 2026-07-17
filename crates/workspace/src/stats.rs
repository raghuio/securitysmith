//! Stats aggregation — count entities and findings across the workspace tree.
//!
//! Walks the tree once, counts clients, projects, engagements, findings.
//! Filters findings by status and severity.

use camino::Utf8PathBuf;
use std::fs;

use crate::Workspace;

/// Stats for a workspace or client scope.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub clients: usize,
    pub projects: usize,
    pub engagements: usize,
    pub findings_total: usize,
    pub findings_open: usize,
    pub findings_by_severity: Vec<(String, usize)>,
    pub engagements_by_status: Vec<(String, usize)>,
    pub clients_by_priority: Vec<(String, usize)>,
    pub projects_by_priority: Vec<(String, usize)>,
    pub open_findings_per_project: Vec<(String, usize)>,
}

/// Compute stats for a workspace.
pub fn compute_workspace_stats(ws: &Workspace) -> Stats {
    let mut stats = Stats::default();
    let mut severity_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut eng_status_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut client_priority_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut project_priority_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut open_findings_per_project: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    // Scan clients
    let entries = match fs::read_dir(&ws.root) {
        Ok(e) => e,
        Err(_) => return stats,
    };

    for client_entry in entries {
        let client_entry = match client_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
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
        // Read and parse client config
        let content = match fs::read_to_string(client_dir.join("config.toml")) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !content.contains("[client]") {
            continue;
        }

        stats.clients += 1;
        // Track client priority
        if let Ok(cfg) = toml::from_str::<crate::entities::ClientConfig>(&content) {
            *client_priority_counts
                .entry(cfg.client.priority)
                .or_insert(0) += 1;
        }
        scan_client(
            &Utf8PathBuf::from_path_buf(client_dir).unwrap_or_default(),
            &mut stats,
            &mut severity_counts,
            &mut eng_status_counts,
            &mut project_priority_counts,
            &mut open_findings_per_project,
        );
    }

    stats.findings_by_severity = severity_counts.into_iter().collect();
    stats.engagements_by_status = eng_status_counts.into_iter().collect();
    stats.clients_by_priority = client_priority_counts.into_iter().collect();
    stats.projects_by_priority = project_priority_counts.into_iter().collect();
    stats.open_findings_per_project = open_findings_per_project.into_iter().collect();
    stats
}

/// Compute stats for a single client.
pub fn compute_client_stats(ws: &Workspace, client_name: &str) -> Stats {
    let mut stats = Stats::default();
    let mut severity_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut eng_status_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut client_priority_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut project_priority_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut open_findings_per_project: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    let client_dir = ws.root.join(client_name);
    if !client_dir.exists() {
        return stats;
    }

    stats.clients = 1;
    // Track client priority
    if let Ok(c) = fs::read_to_string(client_dir.join("config.toml")) {
        if let Ok(cfg) = toml::from_str::<crate::entities::ClientConfig>(&c) {
            *client_priority_counts
                .entry(cfg.client.priority)
                .or_insert(0) += 1;
        }
    }
    scan_client(
        &client_dir,
        &mut stats,
        &mut severity_counts,
        &mut eng_status_counts,
        &mut project_priority_counts,
        &mut open_findings_per_project,
    );
    stats.findings_by_severity = severity_counts.into_iter().collect();
    stats.engagements_by_status = eng_status_counts.into_iter().collect();
    stats.clients_by_priority = client_priority_counts.into_iter().collect();
    stats.projects_by_priority = project_priority_counts.into_iter().collect();
    stats.open_findings_per_project = open_findings_per_project.into_iter().collect();
    stats
}

/// Scan a client directory for projects, engagements, and findings.
fn scan_client(
    client_dir: &Utf8PathBuf,
    stats: &mut Stats,
    severity_counts: &mut std::collections::BTreeMap<String, usize>,
    eng_status_counts: &mut std::collections::BTreeMap<String, usize>,
    project_priority_counts: &mut std::collections::BTreeMap<String, usize>,
    open_findings_per_project: &mut std::collections::BTreeMap<String, usize>,
) {
    let entries = match fs::read_dir(client_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for project_entry in entries {
        let project_entry = match project_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
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

        stats.projects += 1;
        let project_name = project_entry.file_name().to_string_lossy().to_string();

        // Track project priority
        if let Ok(pc) = fs::read_to_string(project_dir.join("config.toml")) {
            if let Ok(pcfg) = toml::from_str::<crate::entities::ProjectConfig>(&pc) {
                *project_priority_counts
                    .entry(pcfg.project.priority)
                    .or_insert(0) += 1;
            }
        }

        // Scan engagements
        let eng_entries = match fs::read_dir(&project_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for eng_entry in eng_entries {
            let eng_entry = match eng_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !eng_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let eng_dir = eng_entry.path();
            if !eng_dir.join("config.toml").exists() {
                continue;
            }

            stats.engagements += 1;

            // Track engagement status
            if let Ok(ec) = fs::read_to_string(eng_dir.join("config.toml")) {
                if let Ok(ecfg) = toml::from_str::<crate::entities::EngagementConfig>(&ec) {
                    *eng_status_counts.entry(ecfg.engagement.status).or_insert(0) += 1;
                }
            }

            // Count findings
            let findings_dir = eng_dir.join("findings");
            if findings_dir.exists() {
                if let Ok(f_entries) = fs::read_dir(&findings_dir) {
                    for f_entry in f_entries.flatten() {
                        let f_path = f_entry.path();
                        if f_path.extension().and_then(|e| e.to_str()) != Some("md") {
                            continue;
                        }
                        stats.findings_total += 1;

                        if let Ok(parsed) = ss_frontmatter::parse_file(&f_path) {
                            let status = parsed
                                .frontmatter
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if status == "open" {
                                stats.findings_open += 1;
                                *open_findings_per_project
                                    .entry(project_name.clone())
                                    .or_insert(0) += 1;
                            }

                            let severity = parsed
                                .frontmatter
                                .get("severity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            *severity_counts.entry(severity).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn stats_empty_workspace() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();
        let stats = compute_workspace_stats(&workspace);
        assert_eq!(stats.clients, 0);
    }

    #[test]
    fn stats_with_data() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");
        ws.create_client("foobar");
        ws.create_project("foobar", "api");
        ws.create_engagement("foobar", "api", "pentest");

        let workspace = Workspace::load(&ws.root).unwrap();
        findings::create_finding(&workspace, "acme/web_app/initial", "XSS", true).unwrap();
        findings::create_finding(&workspace, "acme/web_app/initial", "SQLi", true).unwrap();
        findings::create_finding(&workspace, "foobar/api/pentest", "BOLA", true).unwrap();

        let stats = compute_workspace_stats(&workspace);
        assert_eq!(stats.clients, 2);
        assert_eq!(stats.projects, 2);
        assert_eq!(stats.engagements, 2);
        assert_eq!(stats.findings_total, 3);
        assert_eq!(stats.findings_open, 3); // all open by default
    }

    #[test]
    fn stats_per_client() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");
        ws.create_client("foobar");
        ws.create_project("foobar", "api");
        ws.create_engagement("foobar", "api", "pentest");

        let workspace = Workspace::load(&ws.root).unwrap();
        findings::create_finding(&workspace, "acme/web_app/initial", "XSS", true).unwrap();
        findings::create_finding(&workspace, "foobar/api/pentest", "BOLA", true).unwrap();

        let stats = compute_client_stats(&workspace, "acme");
        assert_eq!(stats.clients, 1);
        assert_eq!(stats.projects, 1);
        assert_eq!(stats.findings_total, 1);

        let stats2 = compute_client_stats(&workspace, "foobar");
        assert_eq!(stats2.findings_total, 1);
    }

    #[test]
    fn stats_finds_manual_client() {
        let ws = TestWorkspace::new();
        // Manually create a client
        let dir = ws.root.join("manual_client");
        fs::create_dir_all(&dir).unwrap();
        let mut config = crate::entities::ClientConfig::default();
        config.client.id = Some(crate::entities::ClientIdSection {
            prefix: "MAN".to_string(),
        });
        let toml = toml::to_string_pretty(&config).unwrap();
        fs::write(dir.join("config.toml"), toml).unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let stats = compute_workspace_stats(&workspace);
        assert_eq!(stats.clients, 1);
    }

    #[test]
    fn stats_performance_50_clients_500_findings() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();

        // Create 50 clients, each with 1 project, 1 engagement, and 10 findings
        for i in 0..50 {
            let client_name = format!("client{:02}", i);
            ws.create_client(&client_name);
            ws.create_project(&client_name, "web_app");
            ws.create_engagement(&client_name, "web_app", "initial");
            for j in 0..10 {
                findings::create_finding(
                    &workspace,
                    &format!("{}/web_app/initial", client_name),
                    &format!("Finding {}", j),
                    true,
                )
                .unwrap();
            }
        }

        let start = std::time::Instant::now();
        let stats = compute_workspace_stats(&workspace);
        let elapsed = start.elapsed();

        assert_eq!(stats.clients, 50);
        assert_eq!(stats.projects, 50);
        assert_eq!(stats.engagements, 50);
        assert_eq!(stats.findings_total, 500);

        assert!(
            elapsed.as_secs() < 1,
            "Stats scan took {:?}, expected under 1 second",
            elapsed
        );
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;
    use crate::Workspace;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn stats_track_engagement_status_and_priority() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");
        let workspace = Workspace::load(&ws.root).unwrap();
        let stats = compute_workspace_stats(&workspace);
        assert!(
            !stats.engagements_by_status.is_empty(),
            "engagements_by_status empty: {:?}",
            stats.engagements_by_status
        );
        assert!(
            !stats.clients_by_priority.is_empty(),
            "clients_by_priority empty: {:?}",
            stats.clients_by_priority
        );
        assert!(
            !stats.projects_by_priority.is_empty(),
            "projects_by_priority empty: {:?}",
            stats.projects_by_priority
        );
    }
}
