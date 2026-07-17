//! Scope operations — open scope.md in editor, show content.
//!
//! Scope is pure Markdown, no frontmatter. The user manages content in their editor.

use camino::Utf8PathBuf;
use std::fs;

use crate::entities::{EntityType, resolve_existing_entity};
use crate::{Workspace, WorkspaceError, atomic_write};

/// Default scope template content. Written to `templates/scope.md` on first use,
/// then the user owns and edits it. No recompilation needed to change it.
/// Read at compile time from `templates/scope.md` — visible, editable, diff-friendly.
const DEFAULT_SCOPE_TEMPLATE: &str = include_str!("../templates/scope.md");

/// Ensure scope.md exists for an engagement. Creates it from the template if missing.
/// Returns the path to scope.md. Does NOT launch an editor.
pub fn ensure_scope_file(
    ws: &Workspace,
    engagement_path: &str,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let path = scope_path(ws, engagement_path)?;

    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let template_path = ws.root.join("templates").join("scope.md");
        let content = if template_path.exists() {
            fs::read_to_string(&template_path)?
        } else {
            // Create the template file so the user can edit it later
            if let Some(parent) = template_path.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write(&template_path, DEFAULT_SCOPE_TEMPLATE.as_bytes())?;
            DEFAULT_SCOPE_TEMPLATE.to_string()
        };
        atomic_write(&path, content.as_bytes())?;
    }

    Ok(path)
}

/// Open scope.md in $EDITOR. Creates the file if it doesn't exist.
pub fn open_scope_editor(ws: &Workspace, engagement_path: &str) -> Result<(), WorkspaceError> {
    let path = ensure_scope_file(ws, engagement_path)?;
    crate::spawn_editor(&path)
}

/// Show scope.md content. Returns None if the file doesn't exist.
pub fn show_scope_content(
    ws: &Workspace,
    engagement_path: &str,
) -> Result<Option<String>, WorkspaceError> {
    let path = scope_path(ws, engagement_path)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(&path)?))
}

/// Get the path to scope.md for an engagement.
fn scope_path(ws: &Workspace, engagement_path: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    let (engagement_dir, entity_type) = resolve_existing_entity(ws, engagement_path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(engagement_path)));
    }

    Ok(engagement_dir.join("scope.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn scope_created_with_template() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        ensure_scope_file(&workspace, "acme/web_app/initial").unwrap();

        let scope_path = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("scope.md");
        assert!(scope_path.exists(), "scope.md should be created");
        let content = fs::read_to_string(&scope_path).unwrap();
        assert!(
            content.contains("# Scope"),
            "scope.md should have template content, got: {content}"
        );
        assert!(content.contains("## In Scope"));
        assert!(content.contains("## Out of Scope"));
        assert!(content.contains("## Rules of Engagement"));

        // Verify template file was created in workspace templates/
        let template_path = ws.root.join("templates").join("scope.md");
        assert!(
            template_path.exists(),
            "templates/scope.md should be created"
        );
    }

    #[test]
    fn show_scope_no_file() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let result = show_scope_content(&workspace, "acme/web_app/initial").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn show_scope_with_content() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        // Manually create scope.md
        let scope_path = ws
            .root
            .join("acme")
            .join("web_app")
            .join("initial")
            .join("scope.md");
        fs::write(&scope_path, "# Scope\n\nIn-scope: web app\n").unwrap();

        let workspace = Workspace::load(&ws.root).unwrap();
        let result = show_scope_content(&workspace, "acme/web_app/initial").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("# Scope"));
    }
}
