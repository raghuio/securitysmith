//! Custom documents — RoE, NDA, proposal, and custom document types.
//!
//! Documents are Markdown files with frontmatter (id, type, status, created, updated)
//! stored in `documents/` directories at client or engagement level.

use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::WorkspaceError;

/// Document types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Roe,
    Nda,
    Proposal,
    Custom,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Roe => "roe",
            Self::Nda => "nda",
            Self::Proposal => "proposal",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "roe" => Some(Self::Roe),
            "nda" => Some(Self::Nda),
            "proposal" => Some(Self::Proposal),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Document status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    Finalized,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Finalized => "finalized",
        }
    }
}

/// Document frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentFrontmatter {
    pub id: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    pub status: String,
    pub created: String,
    pub updated: String,
}

/// A parsed document.
#[derive(Debug, Clone)]
pub struct Document {
    pub frontmatter: DocumentFrontmatter,
    pub body: String,
    pub path: Utf8PathBuf,
}

/// Today's date as YYYY-MM-DD.
fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Derive a slug from a title.
fn slugify(title: &str) -> String {
    let mut result: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    while result.contains("__") {
        result = result.replace("__", "_");
    }
    result.trim_matches('_').to_string()
}

/// Create a new document.
///
/// Creates `documents/<type>_<seq>_<slug>.md` at the given parent path
/// with frontmatter and optional template body. Opens in $EDITOR if requested.
pub fn create_document(
    workspace_root: &Utf8Path,
    parent_path: &Utf8Path,
    title: &str,
    doc_type: DocumentType,
    template_body: Option<&str>,
) -> Result<Document, WorkspaceError> {
    let documents_dir = parent_path.join("documents");
    fs::create_dir_all(documents_dir.as_std_path())?;

    // Generate ID: DOC-<seq>
    let seq = next_document_seq(&documents_dir)?;
    let id = format!("DOC-{seq:03}");

    let slug = slugify(title);
    let filename = format!("doc_{seq:03}_{slug}.md");
    let path = documents_dir.join(&filename);

    // Check symlink escape
    crate::check_symlink_escape(workspace_root, &path)?;

    let now = today();
    let frontmatter = DocumentFrontmatter {
        id: id.clone(),
        doc_type: doc_type.as_str().to_string(),
        status: DocumentStatus::Draft.as_str().to_string(),
        created: now.clone(),
        updated: now,
    };

    let fm_yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|e| WorkspaceError::Io(std::io::Error::other(format!("YAML error: {e}"))))?;

    let body = template_body.map(|t| t.to_string()).unwrap_or_default();
    let content = format!("---\n{fm_yaml}---\n\n# {title}\n\n{body}");

    crate::atomic_write(&path, content.as_bytes())?;

    Ok(Document {
        frontmatter,
        body,
        path,
    })
}

/// Find the next document sequence number.
fn next_document_seq(documents_dir: &Utf8Path) -> Result<usize, WorkspaceError> {
    let mut max_seq = 0;
    if documents_dir.exists() {
        for entry in fs::read_dir(documents_dir.as_std_path())? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("doc_") {
                if let Some(seq_str) = name.strip_prefix("doc_").and_then(|s| s.split('_').next()) {
                    if let Ok(seq) = seq_str.parse::<usize>() {
                        max_seq = max_seq.max(seq);
                    }
                }
            }
        }
    }
    Ok(max_seq + 1)
}

/// Load a document by ID.
pub fn find_document_by_id(
    workspace_root: &Utf8Path,
    doc_id: &str,
) -> Result<Document, WorkspaceError> {
    // Search all documents/ directories in the workspace
    let mut found: Option<Document> = None;
    walk_documents(workspace_root, &mut |doc| {
        if doc.frontmatter.id.eq_ignore_ascii_case(doc_id) {
            found = Some(doc.clone());
        }
    })?;

    found.ok_or_else(|| WorkspaceError::NotFound(Utf8PathBuf::from(doc_id)))
}

/// List documents at a specific path level.
pub fn list_documents(parent_path: &Utf8Path) -> Result<Vec<Document>, WorkspaceError> {
    let documents_dir = parent_path.join("documents");
    if !documents_dir.exists() {
        return Ok(Vec::new());
    }

    let mut docs = Vec::new();
    for entry in fs::read_dir(documents_dir.as_std_path())? {
        let entry = entry?;
        let path = camino::Utf8PathBuf::from_path_buf(entry.path()).map_err(|p| {
            WorkspaceError::Io(std::io::Error::other(format!("non-UTF8 path: {p:?}")))
        })?;

        if path.extension() != Some("md") || !path.is_file() {
            continue;
        }

        let content = fs::read_to_string(path.as_std_path())?;
        if let Ok(doc) = parse_document(&content, &path) {
            docs.push(doc);
        }
    }

    docs.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    Ok(docs)
}

/// Parse a document from Markdown content.
fn parse_document(content: &str, path: &Utf8Path) -> Result<Document, WorkspaceError> {
    let parsed = ss_frontmatter::parse(content).map_err(WorkspaceError::Frontmatter)?;

    let frontmatter: DocumentFrontmatter = if parsed.frontmatter.is_null() {
        DocumentFrontmatter {
            id: String::new(),
            doc_type: "custom".to_string(),
            status: "draft".to_string(),
            created: today(),
            updated: today(),
        }
    } else {
        serde_yaml::from_value(parsed.frontmatter.clone())
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(format!("YAML error: {e}"))))?
    };

    Ok(Document {
        frontmatter,
        body: parsed.body,
        path: path.to_path_buf(),
    })
}

/// Finalize a document (set status to finalized — read-only).
pub fn finalize_document(workspace_root: &Utf8Path, doc_id: &str) -> Result<(), WorkspaceError> {
    let doc = find_document_by_id(workspace_root, doc_id)?;
    if doc.frontmatter.status == DocumentStatus::Finalized.as_str() {
        return Ok(()); // Already finalized
    }
    update_status(&doc, DocumentStatus::Finalized)
}

/// Unlock a document (revert to draft — editable).
pub fn unlock_document(workspace_root: &Utf8Path, doc_id: &str) -> Result<(), WorkspaceError> {
    let doc = find_document_by_id(workspace_root, doc_id)?;
    if doc.frontmatter.status == DocumentStatus::Draft.as_str() {
        return Ok(()); // Already draft
    }
    update_status(&doc, DocumentStatus::Draft)
}

/// Update document status.
fn update_status(doc: &Document, status: DocumentStatus) -> Result<(), WorkspaceError> {
    let content = fs::read_to_string(doc.path.as_std_path())?;
    let parsed = ss_frontmatter::parse(&content)?;

    let mut fm: serde_yaml::Value = parsed.frontmatter;
    if let serde_yaml::Value::Mapping(ref mut map) = fm {
        map.insert(
            serde_yaml::Value::String("status".to_string()),
            serde_yaml::Value::String(status.as_str().to_string()),
        );
        map.insert(
            serde_yaml::Value::String("updated".to_string()),
            serde_yaml::Value::String(today()),
        );
    }

    let fm_yaml = serde_yaml::to_string(&fm)
        .map_err(|e| WorkspaceError::Io(std::io::Error::other(format!("YAML error: {e}"))))?;
    let new_content = format!("---\n{fm_yaml}---\n\n{}", parsed.body);

    crate::atomic_write(&doc.path, new_content.as_bytes())?;
    Ok(())
}

/// Remove a document (to OS trash).
pub fn remove_document(workspace_root: &Utf8Path, doc_id: &str) -> Result<(), WorkspaceError> {
    let doc = find_document_by_id(workspace_root, doc_id)?;
    trash::delete(doc.path.as_std_path())
        .map_err(|e| WorkspaceError::Io(std::io::Error::other(format!("Trash error: {e}"))))?;
    Ok(())
}

/// Walk all documents in the workspace.
fn walk_documents<F: FnMut(&Document)>(dir: &Utf8Path, f: &mut F) -> Result<(), WorkspaceError> {
    let entries = match fs::read_dir(dir.as_std_path()) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = camino::Utf8PathBuf::from_path_buf(entry.path()).map_err(|p| {
            WorkspaceError::Io(std::io::Error::other(format!("non-UTF8 path: {p:?}")))
        })?;

        if path.is_dir() {
            // Check if this is a documents/ directory
            if path.file_name() == Some("documents") {
                for doc in list_documents(path.parent().unwrap_or(dir))? {
                    f(&doc);
                }
            } else {
                // Recurse into subdirectories
                walk_documents(&path, f)?;
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
    fn create_and_find_document() {
        let ws = TestWorkspace::new();
        let client = ws.create_client("acme");

        let doc = create_document(
            &ws.root,
            &client,
            "Rules of Engagement",
            DocumentType::Roe,
            None,
        )
        .unwrap();
        assert_eq!(doc.frontmatter.id, "DOC-001");
        assert_eq!(doc.frontmatter.doc_type, "roe");
        assert_eq!(doc.frontmatter.status, "draft");

        let found = find_document_by_id(&ws.root, "DOC-001").unwrap();
        assert_eq!(found.frontmatter.id, "DOC-001");
    }

    #[test]
    fn finalize_and_unlock() {
        let ws = TestWorkspace::new();
        let client = ws.create_client("acme");

        create_document(&ws.root, &client, "NDA", DocumentType::Nda, None).unwrap();

        finalize_document(&ws.root, "DOC-001").unwrap();
        let doc = find_document_by_id(&ws.root, "DOC-001").unwrap();
        assert_eq!(doc.frontmatter.status, "finalized");

        unlock_document(&ws.root, "DOC-001").unwrap();
        let doc = find_document_by_id(&ws.root, "DOC-001").unwrap();
        assert_eq!(doc.frontmatter.status, "draft");
    }

    #[test]
    fn finalize_already_finalized() {
        let ws = TestWorkspace::new();
        let client = ws.create_client("acme");
        create_document(&ws.root, &client, "NDA", DocumentType::Nda, None).unwrap();
        finalize_document(&ws.root, "DOC-001").unwrap();
        // Second finalize should be a no-op
        finalize_document(&ws.root, "DOC-001").unwrap();
    }

    #[test]
    fn test_list_documents() {
        let ws = TestWorkspace::new();
        let client = ws.create_client("acme");
        create_document(&ws.root, &client, "RoE", DocumentType::Roe, None).unwrap();
        create_document(&ws.root, &client, "NDA", DocumentType::Nda, None).unwrap();

        let docs = list_documents(&client).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn document_type_parse() {
        assert_eq!(DocumentType::parse("roe"), Some(DocumentType::Roe));
        assert_eq!(DocumentType::parse("nda"), Some(DocumentType::Nda));
        assert_eq!(
            DocumentType::parse("proposal"),
            Some(DocumentType::Proposal)
        );
        assert_eq!(DocumentType::parse("custom"), Some(DocumentType::Custom));
        assert_eq!(DocumentType::parse("invalid"), None);
    }

    #[test]
    fn slugify_works() {
        assert_eq!(slugify("Rules of Engagement"), "rules_of_engagement");
        assert_eq!(slugify("NDA"), "nda");
        assert_eq!(slugify("Test!@#Title"), "test_title");
    }
}
