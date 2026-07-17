//! Template system — built-in defaults, workspace overrides, lookup, CRUD.
//!
//! Template priority: workspace templates/ > built-in defaults > none.
//! Templates are pure Markdown, no frontmatter. The filename is the identifier.

use camino::Utf8PathBuf;
use std::fs;

use crate::{Workspace, WorkspaceError};

pub const TEMPLATE_DIR: &str = "templates";
pub const TEMPLATE_TYPES: &[&str] = &["finding", "report", "sow", "requirement"];

/// Built-in default templates compiled into the binary from Markdown files.
/// These are read at compile time via `include_str!` — visible, editable, diff-friendly.
/// Workspace templates in `templates/` override these at runtime.
const FINDING_TEMPLATE: &str = include_str!("../templates/finding.md");
const REQUIREMENT_TEMPLATE: &str = include_str!("../templates/requirement.md");
const REPORT_TEMPLATE: &str = include_str!("../templates/report.md");
const SOW_TEMPLATE: &str = include_str!("../templates/sow.md");

pub fn builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "finding" => Some(FINDING_TEMPLATE),
        "requirement" => Some(REQUIREMENT_TEMPLATE),
        "report" => Some(REPORT_TEMPLATE),
        "sow" => Some(SOW_TEMPLATE),
        _ => None,
    }
}

fn validate_template_name(name: &str) -> Result<(), WorkspaceError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(WorkspaceError::InvalidName(
            "Template names may contain lowercase letters, digits, underscores, and hyphens."
                .to_string(),
        ));
    }
    Ok(())
}

/// Get a template by type name.
/// Priority: workspace templates/ > built-in > none.
/// Checks both `.md` and `.toml` extensions.
pub fn get_template(ws: &Workspace, name: &str) -> Result<Option<String>, WorkspaceError> {
    validate_template_name(name)?;

    let templates_dir = ws.root.join(TEMPLATE_DIR);
    // Check .md then .toml
    for ext in &["md", "toml"] {
        let ws_template_path = templates_dir.join(format!("{}.{}", name, ext));
        if ws_template_path.exists() {
            return Ok(Some(fs::read_to_string(&ws_template_path)?));
        }
    }

    // Try built-in content template
    if let Some(content) = builtin_template(name) {
        return Ok(Some(content.to_string()));
    }
    // Try built-in config template
    if let Some(content) = crate::entities::builtin_config_template(name) {
        return Ok(Some(content.to_string()));
    }
    Ok(None)
}

/// A template entry for listing.
#[derive(Debug, Clone)]
pub struct TemplateEntry {
    pub name: String,
    pub source: TemplateSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateSource {
    Workspace,
    Builtin,
}

/// List all templates: workspace + built-in.
/// Includes both `.md` content templates and `.toml` config templates.
pub fn list_templates(ws: &Workspace) -> Result<Vec<TemplateEntry>, WorkspaceError> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Workspace templates (.md and .toml)
    let templates_dir = ws.root.join(TEMPLATE_DIR);
    if templates_dir.exists() {
        for entry in fs::read_dir(&templates_dir)? {
            let entry = entry?;
            let fname = entry.file_name();
            let fname = fname.to_string_lossy().to_string();
            // Strip .md or .toml suffix
            let name = fname
                .strip_suffix(".md")
                .or_else(|| fname.strip_suffix(".toml"))
                .map(|s| s.to_string());
            if let Some(name) = name {
                entries.push(TemplateEntry {
                    name: name.clone(),
                    source: TemplateSource::Workspace,
                });
                seen.insert(name);
            }
        }
    }

    // Built-in content templates (.md)
    for &name in TEMPLATE_TYPES {
        if !seen.contains(name) {
            entries.push(TemplateEntry {
                name: name.to_string(),
                source: TemplateSource::Builtin,
            });
            seen.insert(name.to_string());
        }
    }

    // Built-in config templates (.toml)
    for &name in crate::entities::CONFIG_TEMPLATE_TYPES {
        if !seen.contains(name) {
            entries.push(TemplateEntry {
                name: name.to_string(),
                source: TemplateSource::Builtin,
            });
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Create a workspace template. Copies built-in default as starting point if available.
/// Supports both `.md` content templates (finding, report, sow, requirement)
/// and `.toml` config templates (workspace, client, project, engagement).
pub fn create_template(ws: &Workspace, name: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_template_name(name)?;

    // Determine if this is a content template (.md) or config template (.toml)
    let is_config_template = crate::entities::CONFIG_TEMPLATE_TYPES.contains(&name);
    let is_content_template = TEMPLATE_TYPES.contains(&name);

    if !is_config_template && !is_content_template {
        let mut all_types = TEMPLATE_TYPES.to_vec();
        all_types.extend_from_slice(crate::entities::CONFIG_TEMPLATE_TYPES);
        return Err(WorkspaceError::InvalidName(format!(
            "Unknown template type: {}. Valid types: {}",
            name,
            all_types.join(", ")
        )));
    }

    let extension = if is_config_template { "toml" } else { "md" };
    let templates_dir = ws.root.join(TEMPLATE_DIR);
    let file_path = templates_dir.join(format!("{}.{}", name, extension));

    if file_path.exists() {
        return Err(WorkspaceError::AlreadyExists(file_path));
    }

    fs::create_dir_all(&templates_dir)?;

    // Copy built-in default as starting point
    let content = if is_config_template {
        crate::entities::builtin_config_template(name)
            .unwrap_or("")
            .to_string()
    } else {
        builtin_template(name).unwrap_or("").to_string()
    };
    crate::atomic_write(&file_path, content.as_bytes())?;

    Ok(file_path)
}

/// Show a template's content.
/// Checks both `.md` and `.toml` extensions for workspace templates.
pub fn show_template(ws: &Workspace, name: &str) -> Result<String, WorkspaceError> {
    validate_template_name(name)?;

    // Try workspace first — check .md then .toml
    let templates_dir = ws.root.join(TEMPLATE_DIR);
    for ext in &["md", "toml"] {
        let ws_path = templates_dir.join(format!("{}.{}", name, ext));
        if ws_path.exists() {
            return Ok(fs::read_to_string(&ws_path)?);
        }
    }

    // Try built-in content template (.md)
    if let Some(content) = builtin_template(name) {
        return Ok(content.to_string());
    }
    // Try built-in config template (.toml)
    if let Some(content) = crate::entities::builtin_config_template(name) {
        return Ok(content.to_string());
    }
    Err(WorkspaceError::NotFound(Utf8PathBuf::from(format!(
        "templates/{}",
        name
    ))))
}

/// Open a workspace template in $EDITOR.
/// Checks both `.md` and `.toml` extensions.
pub fn edit_template(ws: &Workspace, name: &str) -> Result<(), WorkspaceError> {
    validate_template_name(name)?;
    let templates_dir = ws.root.join(TEMPLATE_DIR);
    for ext in &["md", "toml"] {
        let ws_path = templates_dir.join(format!("{}.{}", name, ext));
        if ws_path.exists() {
            return crate::spawn_editor(&ws_path);
        }
    }
    Err(WorkspaceError::NotFound(
        templates_dir.join(format!("{}.md", name)),
    ))
}

/// Remove a workspace template. Cannot remove built-in templates.
/// Checks both `.md` and `.toml` extensions.
pub fn remove_template(ws: &Workspace, name: &str) -> Result<crate::RemovalMethod, WorkspaceError> {
    validate_template_name(name)?;
    let templates_dir = ws.root.join(TEMPLATE_DIR);
    for ext in &["md", "toml"] {
        let ws_path = templates_dir.join(format!("{}.{}", name, ext));
        if ws_path.exists() {
            return crate::trash_or_delete(&ws_path);
        }
    }
    Err(WorkspaceError::NotFound(
        templates_dir.join(format!("{}.md", name)),
    ))
}

/// Check if a template is a workspace template (not built-in).
/// Checks both `.md` and `.toml` extensions.
pub fn is_workspace_template(ws: &Workspace, name: &str) -> bool {
    validate_template_name(name).is_ok()
        && (ws
            .root
            .join(TEMPLATE_DIR)
            .join(format!("{}.md", name))
            .exists()
            || ws
                .root
                .join(TEMPLATE_DIR)
                .join(format!("{}.toml", name))
                .exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn get_builtin_finding_template() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();
        let template = get_template(&workspace, "finding").unwrap();
        assert!(template.is_some());
        assert!(template.unwrap().contains("{{title}}"));
    }

    #[test]
    fn workspace_template_overrides_builtin() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();

        // Create workspace template
        let templates_dir = ws.root.join("templates");
        fs::create_dir_all(&templates_dir).unwrap();
        fs::write(templates_dir.join("finding.md"), "# Custom Finding\n").unwrap();

        let template = get_template(&workspace, "finding").unwrap().unwrap();
        assert!(template.contains("Custom Finding"));
    }

    #[test]
    fn list_templates_shows_both() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();

        // Add a workspace template
        let templates_dir = ws.root.join("templates");
        fs::create_dir_all(&templates_dir).unwrap();
        fs::write(templates_dir.join("finding.md"), "# Custom\n").unwrap();

        let entries = list_templates(&workspace).unwrap();
        let workspace_count = entries
            .iter()
            .filter(|e| e.source == TemplateSource::Workspace)
            .count();
        let builtin_count = entries
            .iter()
            .filter(|e| e.source == TemplateSource::Builtin)
            .count();
        assert_eq!(workspace_count, 1);
        assert_eq!(builtin_count, 7); // report, sow, requirement + workspace, client, project, engagement
    }

    #[test]
    fn create_and_remove_template() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();

        let path = create_template(&workspace, "finding").unwrap();
        assert!(path.exists());

        remove_template(&workspace, "finding").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn create_template_unknown_type_rejected() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();
        assert!(create_template(&workspace, "unknown").is_err());
    }

    #[test]
    fn template_paths_reject_traversal() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();
        let secret = ws.root.join("secret.md");
        fs::create_dir_all(ws.root.join("templates")).unwrap();
        fs::write(&secret, "must stay outside templates").unwrap();

        assert!(show_template(&workspace, "../secret").is_err());
        assert!(remove_template(&workspace, "../secret").is_err());
        assert!(secret.exists());
    }

    #[test]
    fn show_builtin_template() {
        let ws = TestWorkspace::new();
        let workspace = Workspace::load(&ws.root).unwrap();
        let content = show_template(&workspace, "report").unwrap();
        assert!(content.contains("Security Assessment Report"));
    }
}
