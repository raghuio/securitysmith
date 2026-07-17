//! Requirement model — create, show, update, list, remove.
//!
//! Requirements are individual Markdown files with YAML frontmatter (id, status, created, updated).
//! All other content is in the Markdown body.

use camino::Utf8PathBuf;
use std::fs;

use crate::entities;
use crate::{Workspace, WorkspaceError};

/// Create a new requirement under an engagement.
pub fn create_requirement(
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

    let project_dir = ws.root.join(segments[0]).join(segments[1]);

    // Increment requirement sequence and build ID
    let req_id = entities::increment_requirement_sequence(&project_dir)?;

    // Create requirements/ directory if needed
    let requirements_dir = engagement_dir.join("requirements");
    fs::create_dir_all(&requirements_dir)?;

    // Build filename
    let slug = crate::slug_from_title(title);
    let filename_id = req_id.to_lowercase().replace('-', "_");
    let filename = format!("{}_{}.md", filename_id, slug);
    let file_path = requirements_dir.join(&filename);

    // Build frontmatter
    let today = crate::utc_today();
    let mut fm = serde_yaml::Mapping::new();
    fm.insert(
        serde_yaml::Value::String("id".into()),
        serde_yaml::Value::String(req_id.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("status".into()),
        serde_yaml::Value::String("open".into()),
    );
    fm.insert(
        serde_yaml::Value::String("created".into()),
        serde_yaml::Value::String(today.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("updated".into()),
        serde_yaml::Value::String(today),
    );

    // Build body
    let body = if no_template {
        format!("# {}\n\n", title)
    } else {
        match crate::templates::get_template(ws, "requirement")? {
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

/// Find a requirement file by its ID across the workspace.
pub fn find_requirement_file(ws: &Workspace, req_id: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    let target_lower = req_id.to_lowercase().replace('-', "_");

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
                let req_dir = eng_dir.join("requirements");
                if !req_dir.exists() {
                    continue;
                }
                for f_entry in fs::read_dir(&req_dir)? {
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
    Err(WorkspaceError::NotFound(Utf8PathBuf::from(req_id)))
}

/// Show requirement content.
pub fn show_requirement(ws: &Workspace, req_id: &str) -> Result<String, WorkspaceError> {
    let path = find_requirement_file(ws, req_id)?;
    Ok(fs::read_to_string(&path)?)
}

/// Update a requirement's status in frontmatter.
pub fn update_requirement_status(
    ws: &Workspace,
    req_id: &str,
    status: &str,
) -> Result<(), WorkspaceError> {
    let path = find_requirement_file(ws, req_id)?;

    // Validate state machine transition
    let parsed = ss_frontmatter::parse_file(path.as_std_path())?;
    let current_status = parsed
        .frontmatter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !current_status.is_empty()
        && !crate::entities::is_valid_requirement_transition(current_status, status)
    {
        return Err(WorkspaceError::InvalidStatusSeverity(format!(
            "Invalid requirement status transition: {} -> {}",
            current_status, status
        )));
    }

    ss_frontmatter::update_field(
        path.as_std_path(),
        "status",
        &serde_yaml::Value::String(status.to_string()),
    )?;
    Ok(())
}

/// List requirements in an engagement directory.
#[derive(Debug, Clone)]
pub struct RequirementEntry {
    pub id: String,
    pub status: String,
    pub filename: String,
}

pub fn list_requirements(
    engagement_dir: &Utf8PathBuf,
) -> Result<Vec<RequirementEntry>, WorkspaceError> {
    let req_dir = engagement_dir.join("requirements");
    if !req_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for f_entry in fs::read_dir(&req_dir)? {
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

        entries.push(RequirementEntry {
            id,
            status,
            filename: fname,
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

/// Remove a requirement by ID. Moves to OS trash. Caller handles confirmation.
pub fn remove_requirement(
    ws: &Workspace,
    req_id: &str,
) -> Result<crate::RemovalMethod, WorkspaceError> {
    let path = find_requirement_file(ws, req_id)?;
    crate::trash_or_delete(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn create_and_find_requirement() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let path =
            create_requirement(&workspace, "acme/web_app/initial", "Test auth", true).unwrap();
        assert!(path.exists());

        let found = find_requirement_file(&workspace, "REQ-001").unwrap();
        assert_eq!(found, path);
    }

    #[test]
    fn update_req_status() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Test auth", true).unwrap();

        update_requirement_status(&workspace, "REQ-001", "in_progress").unwrap();

        let content = show_requirement(&workspace, "REQ-001").unwrap();
        assert!(content.contains("in_progress"));
    }

    #[test]
    fn list_requirements_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Req One", true).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Req Two", true).unwrap();

        let eng_dir = ws.root.join("acme").join("web_app").join("initial");
        let reqs = list_requirements(&eng_dir).unwrap();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].id, "REQ-001");
        assert_eq!(reqs[1].id, "REQ-002");
    }

    #[test]
    fn remove_requirement_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Test", true).unwrap();
        assert!(find_requirement_file(&workspace, "REQ-001").is_ok());

        remove_requirement(&workspace, "REQ-001").unwrap();
        assert!(find_requirement_file(&workspace, "REQ-001").is_err());
    }

    #[test]
    fn invalid_status_transition_rejected() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Test", true).unwrap();
        // REQ-001 starts as "open". "verified" is only reachable from "in_progress".
        let result = update_requirement_status(&workspace, "REQ-001", "verified");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, WorkspaceError::InvalidStatusSeverity(_)));
    }

    #[test]
    fn same_status_transition_is_noop() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_requirement(&workspace, "acme/web_app/initial", "Test", true).unwrap();
        // REQ-001 starts as "open". Setting it to "open" again should succeed (no-op).
        update_requirement_status(&workspace, "REQ-001", "open").unwrap();

        let content = show_requirement(&workspace, "REQ-001").unwrap();
        assert!(content.contains("status: open"));
    }
}
