//! Note model — lightweight create, list, remove.
//!
//! Notes are timestamped Markdown files with minimal frontmatter (id, created, updated).
//! Body has title as H1, message, free-form notes.

use camino::Utf8PathBuf;
use std::fs;

use crate::entities::{EntityType, resolve_existing_entity};
use crate::{Workspace, WorkspaceError};

/// Create a quick note with a message.
/// Returns the file path.
pub fn create_note(
    ws: &Workspace,
    engagement_path: &str,
    message: &str,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let (engagement_dir, entity_type) = resolve_existing_entity(ws, engagement_path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(engagement_path)));
    }

    // Create notes/ directory
    let notes_dir = engagement_dir.join("notes");
    fs::create_dir_all(&notes_dir)?;

    // Generate note ID by scanning existing notes
    let note_num = next_note_number(&notes_dir)?;
    let note_id = format!("NOTE-{:03}", note_num);

    // Build filename: <date>_<slug>.md
    let today = crate::utc_today();
    let date_prefix = today.replace('-', "_");
    let slug = crate::slug_from_title(message);
    // Truncate slug to reasonable length
    let slug: String = slug.chars().take(40).collect();
    let filename = format!("{}_{}.md", date_prefix, slug);
    let file_path = notes_dir.join(&filename);

    // Build frontmatter
    let mut fm = serde_yaml::Mapping::new();
    fm.insert(
        serde_yaml::Value::String("id".into()),
        serde_yaml::Value::String(note_id),
    );
    fm.insert(
        serde_yaml::Value::String("created".into()),
        serde_yaml::Value::String(today.clone()),
    );
    fm.insert(
        serde_yaml::Value::String("updated".into()),
        serde_yaml::Value::String(today),
    );

    // Body: title as H1 + message
    let body = format!("# {}\n", message);

    ss_frontmatter::write_file(
        file_path.as_std_path(),
        &serde_yaml::Value::Mapping(fm),
        &body,
    )?;

    Ok(file_path)
}

/// Find the next note number by scanning existing notes.
fn next_note_number(notes_dir: &Utf8PathBuf) -> Result<u32, WorkspaceError> {
    let mut max_num = 0u32;
    if !notes_dir.exists() {
        return Ok(1);
    }
    for entry in fs::read_dir(notes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(parsed) = ss_frontmatter::parse_file(&path) {
            if let Some(id) = parsed.frontmatter.get("id").and_then(|v| v.as_str()) {
                if let Some(num_str) = id.strip_prefix("NOTE-") {
                    if let Ok(n) = num_str.parse::<u32>() {
                        max_num = max_num.max(n);
                    }
                }
            }
        }
    }
    Ok(max_num + 1)
}

/// List notes in an engagement directory.
#[derive(Debug, Clone)]
pub struct NoteEntry {
    pub id: String,
    pub filename: String,
}

pub fn list_notes(engagement_dir: &Utf8PathBuf) -> Result<Vec<NoteEntry>, WorkspaceError> {
    let notes_dir = engagement_dir.join("notes");
    if !notes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for f_entry in fs::read_dir(&notes_dir)? {
        let f_entry = f_entry?;
        let path = f_entry.path();
        let fname = f_entry.file_name().to_string_lossy().to_string();
        if !fname.ends_with(".md") {
            continue;
        }

        let parsed = ss_frontmatter::parse_file(&path)?;
        let id = parsed
            .frontmatter
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        entries.push(NoteEntry {
            id,
            filename: fname,
        });
    }

    entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(entries)
}

/// Remove a note by its slug (filename without .md).
/// The path is engagement_path/notes/<slug>.
pub fn remove_note(
    ws: &Workspace,
    engagement_path: &str,
    note_slug: &str,
) -> Result<crate::RemovalMethod, WorkspaceError> {
    let (engagement_dir, entity_type) = resolve_existing_entity(ws, engagement_path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(engagement_path)));
    }

    let slug = note_slug.strip_suffix(".md").unwrap_or(note_slug);
    if slug.is_empty()
        || slug == "."
        || slug == ".."
        || slug.chars().any(|c| matches!(c, '/' | '\\' | ':' | '\0'))
    {
        return Err(WorkspaceError::InvalidName(
            "Note names must be a single filename.".to_string(),
        ));
    }

    let notes_dir = engagement_dir.join("notes");
    let target = notes_dir.join(format!("{}.md", slug));

    if !target.exists() {
        return Err(WorkspaceError::NotFound(target));
    }

    crate::trash_or_delete(&target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn create_note_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let path = create_note(
            &workspace,
            "acme/web_app/initial",
            "Remember to test rate limits",
        )
        .unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("NOTE-001"));
        assert!(content.contains("Remember to test rate limits"));
    }

    #[test]
    fn list_notes_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_note(&workspace, "acme/web_app/initial", "First note").unwrap();
        create_note(&workspace, "acme/web_app/initial", "Second note").unwrap();

        let eng_dir = ws.root.join("acme").join("web_app").join("initial");
        let notes = list_notes(&eng_dir).unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn remove_note_rejects_traversal() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        let project_file = ws.root.join("acme/web_app/config.md");
        fs::create_dir_all(ws.root.join("acme/web_app/initial/notes")).unwrap();
        fs::write(&project_file, "must stay outside notes").unwrap();

        assert!(remove_note(&workspace, "acme/web_app/initial", "../../config").is_err());
        assert!(project_file.exists());
    }

    #[test]
    fn remove_note_works() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        ws.create_engagement("acme", "web_app", "initial");

        let workspace = Workspace::load(&ws.root).unwrap();
        create_note(&workspace, "acme/web_app/initial", "Test note").unwrap();

        let eng_dir = ws.root.join("acme").join("web_app").join("initial");
        let notes = list_notes(&eng_dir).unwrap();
        assert_eq!(notes.len(), 1);

        // Remove by filename
        remove_note(&workspace, "acme/web_app/initial", &notes[0].filename).unwrap();
        let notes_after = list_notes(&eng_dir).unwrap();
        assert!(notes_after.is_empty());
    }
}
