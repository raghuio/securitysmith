//! Methodology checklists — track test case coverage during engagements.

use serde::{Deserialize, Serialize};

use crate::WorkspaceError;

/// Built-in checklist definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistDef {
    pub checklist: ChecklistMeta,
    #[serde(default)]
    pub categories: Vec<ChecklistCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistMeta {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistCategory {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub items: Vec<ChecklistItemDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItemDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// Per-engagement checklist tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChecklistTracking {
    pub tracking: TrackingMeta,
    #[serde(default, rename = "item")]
    pub items: Vec<TrackingItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackingMeta {
    #[serde(default)]
    pub checklist_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackingItem {
    pub id: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub finding_id: String,
    #[serde(default)]
    pub notes: String,
}

fn default_status() -> String {
    "not_started".to_string()
}

/// Valid status values.
pub const VALID_STATUSES: &[&str] = &[
    "not_started",
    "in_progress",
    "tested",
    "not_applicable",
    "finding_created",
    "deferred",
];

/// Get a built-in checklist definition by name.
pub fn get_builtin_checklist(name: &str) -> Option<ChecklistDef> {
    match name {
        "owasp-wstg" => Some(owasp_wstg_subset()),
        _ => None,
    }
}

/// List available built-in checklist names.
pub fn list_builtin_checklists() -> Vec<&'static str> {
    vec!["owasp-wstg"]
}

/// Assign a checklist to an engagement. Creates `checklist.toml`.
pub fn assign_checklist(
    ws: &crate::Workspace,
    engagement_path: &str,
    checklist_name: &str,
) -> Result<usize, WorkspaceError> {
    let def = get_builtin_checklist(checklist_name)
        .ok_or_else(|| WorkspaceError::NotFound(camino::Utf8PathBuf::from(checklist_name)))?;

    let (eng_dir, entity_type) = crate::entities::resolve_existing_entity(ws, engagement_path)?;
    if entity_type != crate::entities::EntityType::Engagement {
        return Err(WorkspaceError::NotFound(eng_dir));
    }

    let tracking = ChecklistTracking {
        tracking: TrackingMeta {
            checklist_name: checklist_name.to_string(),
        },
        items: def
            .categories
            .iter()
            .flat_map(|c| {
                c.items.iter().map(|item| TrackingItem {
                    id: item.id.clone(),
                    status: default_status(),
                    ..Default::default()
                })
            })
            .collect(),
    };

    let toml = toml::to_string_pretty(&tracking)?;
    crate::atomic_write(&eng_dir.join("checklist.toml"), toml.as_bytes())?;

    Ok(tracking.items.len())
}

/// Load checklist tracking for an engagement.
pub fn load_tracking(
    ws: &crate::Workspace,
    engagement_path: &str,
) -> Result<ChecklistTracking, WorkspaceError> {
    let (eng_dir, _) = crate::entities::resolve_existing_entity(ws, engagement_path)?;
    let path = eng_dir.join("checklist.toml");
    if !path.exists() {
        return Err(WorkspaceError::NotFound(path));
    }
    let content = std::fs::read_to_string(path.as_std_path())?;
    let tracking: ChecklistTracking = toml::from_str(&content)?;
    Ok(tracking)
}

/// Update item status.
pub fn update_item_status(
    ws: &crate::Workspace,
    engagement_path: &str,
    item_id: &str,
    status: &str,
) -> Result<(), WorkspaceError> {
    if !VALID_STATUSES.contains(&status) {
        return Err(WorkspaceError::InvalidStatusSeverity(format!(
            "'{}' is not a valid checklist status. Use: {}.",
            status,
            VALID_STATUSES.join(", ")
        )));
    }

    let (eng_dir, _) = crate::entities::resolve_existing_entity(ws, engagement_path)?;
    let path = eng_dir.join("checklist.toml");
    let content = std::fs::read_to_string(path.as_std_path())?;
    let mut tracking: ChecklistTracking = toml::from_str(&content)?;

    let item = tracking
        .items
        .iter_mut()
        .find(|i| i.id == item_id)
        .ok_or_else(|| WorkspaceError::NotFound(camino::Utf8PathBuf::from(item_id)))?;

    item.status = status.to_string();
    let toml = toml::to_string_pretty(&tracking)?;
    crate::atomic_write(&path, toml.as_bytes())?;
    Ok(())
}

/// Link a finding to a checklist item.
pub fn link_finding(
    ws: &crate::Workspace,
    engagement_path: &str,
    item_id: &str,
    finding_id: &str,
) -> Result<(), WorkspaceError> {
    let (eng_dir, _) = crate::entities::resolve_existing_entity(ws, engagement_path)?;
    let path = eng_dir.join("checklist.toml");
    let content = std::fs::read_to_string(path.as_std_path())?;
    let mut tracking: ChecklistTracking = toml::from_str(&content)?;

    let item = tracking
        .items
        .iter_mut()
        .find(|i| i.id == item_id)
        .ok_or_else(|| WorkspaceError::NotFound(camino::Utf8PathBuf::from(item_id)))?;

    item.finding_id = finding_id.to_string();
    if item.status == "not_started" {
        item.status = "finding_created".to_string();
    }
    let toml = toml::to_string_pretty(&tracking)?;
    crate::atomic_write(&path, toml.as_bytes())?;
    Ok(())
}

/// Compute coverage percentage.
pub fn compute_coverage(tracking: &ChecklistTracking) -> f64 {
    if tracking.items.is_empty() {
        return 0.0;
    }
    let covered = tracking
        .items
        .iter()
        .filter(|i| {
            matches!(
                i.status.as_str(),
                "tested" | "finding_created" | "not_applicable"
            )
        })
        .count();
    (covered as f64 / tracking.items.len() as f64) * 100.0
}

/// OWASP WSTG v4.2 subset — 15 items across 3 categories.
fn owasp_wstg_subset() -> ChecklistDef {
    ChecklistDef {
        checklist: ChecklistMeta {
            name: "OWASP WSTG v4.2".to_string(),
            version: "4.2".to_string(),
        },
        categories: vec![
            ChecklistCategory {
                name: "Information Gathering".to_string(),
                id: "WSTG-INFO".to_string(),
                items: vec![
                    ChecklistItemDef {
                        id: "WSTG-INFO-01".to_string(),
                        name: "Conduct Search Engine Discovery".to_string(),
                        description: "Test for information leakage in search engines".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-INFO-02".to_string(),
                        name: "Fingerprint Web Server".to_string(),
                        description: "Identify web server technology".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-INFO-03".to_string(),
                        name: "Review Webserver Metafiles".to_string(),
                        description: "Check robots.txt, sitemap.xml".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-INFO-04".to_string(),
                        name: "Enumerate Applications on Webserver".to_string(),
                        description: "Find all apps hosted on the server".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-INFO-05".to_string(),
                        name: "Review Webpage Content for Information Leakage".to_string(),
                        description: "Check HTML comments, source code".to_string(),
                    },
                ],
            },
            ChecklistCategory {
                name: "Configuration and Deploy Management Testing".to_string(),
                id: "WSTG-CONF".to_string(),
                items: vec![
                    ChecklistItemDef {
                        id: "WSTG-CONF-01".to_string(),
                        name: "Test Network Infrastructure Configuration".to_string(),
                        description: "Check network layer config".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-CONF-02".to_string(),
                        name: "Test Application Platform Configuration".to_string(),
                        description: "Check app server config".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-CONF-03".to_string(),
                        name: "Test File Permissions".to_string(),
                        description: "Check sensitive file access".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-CONF-04".to_string(),
                        name: "Test for Old Backup and Unreferenced Files".to_string(),
                        description: "Find backup files".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-CONF-05".to_string(),
                        name: "Enumerate Infrastructure and Application Admin Interfaces"
                            .to_string(),
                        description: "Find admin panels".to_string(),
                    },
                ],
            },
            ChecklistCategory {
                name: "Identity Management Testing".to_string(),
                id: "WSTG-IDM".to_string(),
                items: vec![
                    ChecklistItemDef {
                        id: "WSTG-IDM-01".to_string(),
                        name: "Test User Registration Process".to_string(),
                        description: "Check registration for weaknesses".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-IDM-02".to_string(),
                        name: "Test Account Suspension and Deletion".to_string(),
                        description: "Verify account lifecycle".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-IDM-03".to_string(),
                        name: "Test Account Enumeration and Guessable User Account".to_string(),
                        description: "Check for user enumeration".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-IDM-04".to_string(),
                        name: "Test for Weak or Unenforced Username Policy".to_string(),
                        description: "Check username rules".to_string(),
                    },
                    ChecklistItemDef {
                        id: "WSTG-IDM-05".to_string(),
                        name: "Test for Weak Password Policy".to_string(),
                        description: "Check password requirements".to_string(),
                    },
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn assign_creates_checklist() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        let count = assign_checklist(&ws, "acme/web/initial", "owasp-wstg").unwrap();
        assert_eq!(count, 15);

        let tracking = load_tracking(&ws, "acme/web/initial").unwrap();
        assert_eq!(tracking.items.len(), 15);
        assert!(tracking.items.iter().all(|i| i.status == "not_started"));
    }

    #[test]
    fn update_status_works() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        assign_checklist(&ws, "acme/web/initial", "owasp-wstg").unwrap();
        update_item_status(&ws, "acme/web/initial", "WSTG-INFO-01", "tested").unwrap();

        let tracking = load_tracking(&ws, "acme/web/initial").unwrap();
        let item = tracking
            .items
            .iter()
            .find(|i| i.id == "WSTG-INFO-01")
            .unwrap();
        assert_eq!(item.status, "tested");
    }

    #[test]
    fn link_finding_updates_status() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        assign_checklist(&ws, "acme/web/initial", "owasp-wstg").unwrap();
        link_finding(&ws, "acme/web/initial", "WSTG-INFO-02", "ACME-WEB-001").unwrap();

        let tracking = load_tracking(&ws, "acme/web/initial").unwrap();
        let item = tracking
            .items
            .iter()
            .find(|i| i.id == "WSTG-INFO-02")
            .unwrap();
        assert_eq!(item.finding_id, "ACME-WEB-001");
        assert_eq!(item.status, "finding_created");
    }

    #[test]
    fn coverage_calculation() {
        let tracking = ChecklistTracking {
            tracking: TrackingMeta::default(),
            items: vec![
                TrackingItem {
                    id: "1".into(),
                    status: "tested".into(),
                    ..Default::default()
                },
                TrackingItem {
                    id: "2".into(),
                    status: "not_started".into(),
                    ..Default::default()
                },
                TrackingItem {
                    id: "3".into(),
                    status: "finding_created".into(),
                    ..Default::default()
                },
                TrackingItem {
                    id: "4".into(),
                    status: "not_applicable".into(),
                    ..Default::default()
                },
                TrackingItem {
                    id: "5".into(),
                    status: "in_progress".into(),
                    ..Default::default()
                },
            ],
        };
        let coverage = compute_coverage(&tracking);
        assert_eq!(coverage, 60.0); // 3 out of 5
    }

    #[test]
    fn invalid_status_rejected() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        assign_checklist(&ws, "acme/web/initial", "owasp-wstg").unwrap();
        let result = update_item_status(&ws, "acme/web/initial", "WSTG-INFO-01", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn builtin_checklists_listed() {
        let list = list_builtin_checklists();
        assert!(list.contains(&"owasp-wstg"));
    }
}
