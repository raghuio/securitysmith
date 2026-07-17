//! Workspace search — text search across all Markdown files.

use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

use crate::WorkspaceError;

/// Maximum results to return.
const MAX_RESULTS: usize = 50;

/// A search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entity_type: String,
    pub file_path: String,
    pub line_number: usize,
    pub matching_line: String,
}

/// Search all Markdown files in the workspace for a query string.
/// Returns results grouped by entity type.
pub fn search_workspace(
    workspace_root: &Utf8Path,
    query: &str,
    entity_type_filter: Option<&str>,
    client_filter: Option<&str>,
) -> Result<Vec<SearchResult>, WorkspaceError> {
    let mut results = Vec::new();
    walk_and_search(
        workspace_root,
        workspace_root,
        query,
        entity_type_filter,
        client_filter,
        &mut results,
    )?;
    Ok(results)
}

/// Recursively walk a directory and search Markdown files.
fn walk_and_search(
    workspace_root: &Utf8Path,
    current_dir: &Utf8Path,
    query: &str,
    entity_type_filter: Option<&str>,
    client_filter: Option<&str>,
    results: &mut Vec<SearchResult>,
) -> Result<(), WorkspaceError> {
    if results.len() >= MAX_RESULTS {
        return Ok(());
    }

    let entries = match fs::read_dir(current_dir.as_std_path()) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        if results.len() >= MAX_RESULTS {
            return Ok(());
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = match Utf8PathBuf::from_path_buf(entry.path()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Skip hidden files (like .credentials.enc)
        if path
            .file_name()
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        // Symlink escape prevention — skip symlinks
        if path.is_symlink() {
            if let Ok(target) = fs::canonicalize(path.as_std_path()) {
                let ws_canonical = fs::canonicalize(workspace_root.as_std_path())?;
                if !target.starts_with(&ws_canonical) {
                    continue; // Symlink escapes workspace — skip
                }
            } else {
                continue; // Broken symlink — skip
            }
        }

        if path.is_dir() {
            walk_and_search(
                workspace_root,
                &path,
                query,
                entity_type_filter,
                client_filter,
                results,
            )?;
        } else if path.is_file() && path.extension() == Some("md") {
            let rel = path.strip_prefix(workspace_root).unwrap_or(&path);
            let entity_type = classify_entity(rel);
            let client_name = rel.iter().next().map(|s| s.to_string());

            // Apply filters
            if let Some(filter) = entity_type_filter {
                if entity_type != filter {
                    continue;
                }
            }
            if let Some(filter_client) = client_filter {
                if client_name.as_deref() != Some(filter_client) {
                    continue;
                }
            }

            // Search file content
            if let Ok(content) = fs::read_to_string(path.as_std_path()) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        results.push(SearchResult {
                            entity_type: entity_type.to_string(),
                            file_path: rel.to_string(),
                            line_number: i + 1,
                            matching_line: line.trim().to_string(),
                        });
                        if results.len() >= MAX_RESULTS {
                            return Ok(());
                        }
                        break; // One match per file is enough
                    }
                }
            }
        }
    }

    Ok(())
}

/// Classify entity type based on relative path.
fn classify_entity(rel_path: &Utf8Path) -> &'static str {
    let segments: Vec<&str> = rel_path
        .as_str()
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // templates/ directory
    if segments.first() == Some(&"templates") {
        return "template";
    }

    // Check for known subdirectories
    for seg in &segments {
        match *seg {
            "findings" => return "finding",
            "requirements" => return "requirement",
            "notes" => return "note",
            "documents" => return "document",
            "docs" => return "section",
            _ => {}
        }
    }

    // Check for scope.md
    if segments.last() == Some(&"scope.md") {
        return "scope";
    }

    // Check for config.toml
    if segments.last() == Some(&"config.toml") {
        match segments.len() {
            1 => "workspace",
            2 => "client",
            3 => "project",
            _ => "engagement",
        }
    } else {
        match segments.len() {
            1 => "client",
            2 => "project",
            3 => "engagement",
            _ => "file",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn search_finds_finding() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");
        tw.create_finding(&eng, "ACME-WEB-001", "SQL Injection");

        let results = search_workspace(&tw.root, "SQL Injection", None, None).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.entity_type == "finding"));
    }

    #[test]
    fn search_filter_by_type() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");
        tw.create_finding(&eng, "ACME-WEB-001", "SQL Injection");

        let results =
            search_workspace(&tw.root, "SQL Injection", Some("requirement"), None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_filter_by_client() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_client("other");
        tw.create_project("acme", "web");
        tw.create_project("other", "web");
        let eng1 = tw.create_engagement("acme", "web", "initial");
        let eng2 = tw.create_engagement("other", "web", "initial");
        tw.create_finding(&eng1, "ACME-WEB-001", "UniqueTerm");
        tw.create_finding(&eng2, "OTHER-WEB-001", "UniqueTerm");

        let results = search_workspace(&tw.root, "UniqueTerm", None, Some("acme")).unwrap();
        assert!(results.iter().all(|r| r.file_path.starts_with("acme/")));
    }

    #[test]
    fn search_no_matches() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        let results = search_workspace(&tw.root, "nonexistent_query", None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_case_insensitive() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");
        tw.create_finding(&eng, "ACME-WEB-001", "XSS Vulnerability");

        let results = search_workspace(&tw.root, "xss", None, None).unwrap();
        assert!(!results.is_empty());
    }
}
