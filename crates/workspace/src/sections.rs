//! Document section discovery — modular, reusable Markdown building blocks
//! organized by hierarchy level and engagement type.

use camino::{Utf8Path, Utf8PathBuf};
use std::collections::BTreeMap;
use std::fs;

use crate::WorkspaceError;

/// A discovered document section.
#[derive(Debug, Clone)]
pub struct Section {
    /// Section name (filename without .md extension).
    pub name: String,
    /// Source level: "workspace", "client", "project", or "engagement".
    pub source_level: &'static str,
    /// Filesystem path to the section file.
    pub path: Utf8PathBuf,
    /// Markdown content of the section.
    pub content: String,
}

/// Discover document sections for a target path.
///
/// Scans `docs/` directories at engagement → project → client → workspace levels.
/// At each level, looks in `docs/<engagement_type>/` and `docs/common/`.
/// First match wins — a section found at a lower level overrides the same name at a higher level.
pub fn discover_sections(
    workspace_root: &Utf8Path,
    target_path: &Utf8Path,
    engagement_type: &str,
) -> Result<Vec<Section>, WorkspaceError> {
    let levels = hierarchy_levels(workspace_root, target_path);
    let mut found: BTreeMap<String, Section> = BTreeMap::new();

    // Scan from lowest level (engagement) to highest (workspace).
    // First match wins, so we only insert if the name isn't already found.
    for (level_name, level_dir) in &levels {
        let docs_dir = level_dir.join("docs");
        if !docs_dir.exists() {
            continue;
        }

        // Check type-specific directory first, then common
        let type_dir = docs_dir.join(engagement_type);
        let common_dir = docs_dir.join("common");

        for sub_dir in [type_dir, common_dir] {
            if !sub_dir.exists() {
                continue;
            }
            scan_dir(&sub_dir, level_name, &mut found, workspace_root)?;
        }

        // Also check docs/ directly (non-typed sections at this level)
        scan_dir(&docs_dir, level_name, &mut found, workspace_root)?;
    }

    Ok(found.into_values().collect())
}

/// Resolve hierarchy levels for a target path.
/// Returns levels from engagement (lowest) to workspace (highest).
fn hierarchy_levels<'a>(
    workspace_root: &'a Utf8Path,
    target_path: &'a Utf8Path,
) -> Vec<(&'static str, Utf8PathBuf)> {
    let mut levels = Vec::new();

    // Determine the relative path from workspace root
    let rel = if target_path.is_absolute() {
        target_path
            .strip_prefix(workspace_root)
            .unwrap_or(target_path)
    } else {
        target_path
    };

    let segments: Vec<&str> = rel
        .as_str()
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    // Build cumulative paths for each depth level
    let mut cumulative = workspace_root.to_path_buf();
    for (i, segment) in segments.iter().enumerate() {
        cumulative = cumulative.join(segment);
        let level_name = match i {
            0 => "client",
            1 => "project",
            2 => "engagement",
            _ => "engagement",
        };
        levels.push((level_name, cumulative.clone()));
    }

    // Always include workspace as the highest level
    levels.push(("workspace", workspace_root.to_path_buf()));

    levels
}

/// Scan a directory for .md files and add them to the found map.
fn scan_dir(
    dir: &Utf8Path,
    level_name: &'static str,
    found: &mut BTreeMap<String, Section>,
    workspace_root: &Utf8Path,
) -> Result<(), WorkspaceError> {
    let entries = match fs::read_dir(dir.as_std_path()) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = camino::Utf8PathBuf::from_path_buf(entry.path()).map_err(|p| {
            WorkspaceError::Io(std::io::Error::other(format!("non-UTF8 path: {p:?}")))
        })?;

        // Only .md files
        if path.extension() != Some("md") {
            continue;
        }

        // Skip subdirectories — only flat .md files
        if !path.is_file() {
            continue;
        }

        let name = path.file_stem().unwrap_or("unknown").to_string();

        // Skip if we already found this section at a lower level
        if found.contains_key(&name) {
            continue;
        }

        // Symlink escape check
        crate::check_symlink_escape(workspace_root, &path)?;

        let content = fs::read_to_string(path.as_std_path())?;
        found.insert(
            name.clone(),
            Section {
                name,
                source_level: level_name,
                path,
                content,
            },
        );
    }

    Ok(())
}

/// Load a specific section by type/name path (e.g., "web/methodology").
/// Looks in `docs/<type>/<name>.md` at each hierarchy level.
pub fn load_section_by_path(
    workspace_root: &Utf8Path,
    target_path: &Utf8Path,
    type_and_name: &str,
) -> Result<Option<Section>, WorkspaceError> {
    let parts: Vec<&str> = type_and_name.splitn(2, '/').collect();
    let (type_dir, section_name) = match parts.as_slice() {
        [type_name, section] => (*type_name, *section),
        [name] => ("", *name),
        _ => return Ok(None),
    };

    let levels = hierarchy_levels(workspace_root, target_path);

    for (level_name, level_dir) in &levels {
        let docs_dir = level_dir.join("docs");
        if !docs_dir.exists() {
            continue;
        }

        let candidate = if type_dir.is_empty() {
            // Bare name — check all subdirs
            let type_subdir = docs_dir.join("common").join(format!("{section_name}.md"));
            if type_subdir.exists() {
                type_subdir
            } else {
                docs_dir.join(format!("{section_name}.md"))
            }
        } else {
            docs_dir.join(type_dir).join(format!("{section_name}.md"))
        };

        if candidate.exists() && candidate.is_file() {
            crate::check_symlink_escape(workspace_root, &candidate)?;
            let content = fs::read_to_string(candidate.as_std_path())?;
            return Ok(Some(Section {
                name: if type_dir.is_empty() {
                    section_name.to_string()
                } else {
                    format!("{type_dir}_{section_name}")
                },
                source_level: level_name,
                path: candidate,
                content,
            }));
        }
    }

    Ok(None)
}

/// Assemble final sections dict from discovered sections + user include/exclude flags.
///
/// - If `include` is specified: only include those sections (supports type/name paths).
/// - If `exclude` is specified: remove those sections from the auto-discovered set.
/// - If neither: use all auto-discovered sections.
///
/// Returns a dict of section_name → markdown_content.
pub fn assemble_sections(
    workspace_root: &Utf8Path,
    target_path: &Utf8Path,
    engagement_type: &str,
    include: Option<&str>,
    exclude: Option<&str>,
) -> Result<BTreeMap<String, String>, WorkspaceError> {
    let mut result: BTreeMap<String, String> = BTreeMap::new();

    if let Some(include_list) = include {
        // Explicit include — load each specified section
        for item in include_list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if item.contains('/') {
                // Path like "web/methodology" — load from specific type dir
                if let Some(section) = load_section_by_path(workspace_root, target_path, item)? {
                    result.insert(section.name, section.content);
                }
            } else {
                // Bare name — find in discovered sections
                let discovered = discover_sections(workspace_root, target_path, engagement_type)?;
                if let Some(s) = discovered.iter().find(|s| s.name == item) {
                    result.insert(s.name.clone(), s.content.clone());
                }
            }
        }
    } else {
        // Auto-discover all sections
        let discovered = discover_sections(workspace_root, target_path, engagement_type)?;
        for s in discovered {
            result.insert(s.name, s.content);
        }
    }

    // Apply exclusions
    if let Some(exclude_list) = exclude {
        for item in exclude_list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            result.remove(item);
        }
    }

    Ok(result)
}

/// List all available sections with their source level.
/// Returns formatted strings for display.
pub fn list_sections(
    workspace_root: &Utf8Path,
    target_path: &Utf8Path,
    engagement_type: &str,
) -> Result<Vec<(String, String)>, WorkspaceError> {
    let discovered = discover_sections(workspace_root, target_path, engagement_type)?;
    Ok(discovered
        .iter()
        .map(|s| {
            let source = format!("[{}: {}]", s.source_level, s.path);
            (s.name.clone(), source)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn discover_sections_empty_workspace() {
        let ws = TestWorkspace::new();
        let target = ws.root.join("acme").join("web").join("initial");
        let sections = discover_sections(&ws.root, &target, "web").unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn discover_sections_at_workspace_level() {
        let ws = TestWorkspace::new();
        let docs = ws.root.join("docs").join("web");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("methodology.md"), "# Methodology\n\nOWASP").unwrap();

        let target = ws.root.join("acme").join("web").join("initial");
        let sections = discover_sections(&ws.root, &target, "web").unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "methodology");
        assert_eq!(sections[0].source_level, "workspace");
    }

    #[test]
    fn discover_sections_type_filtering() {
        let ws = TestWorkspace::new();

        let web_docs = ws.root.join("docs").join("web");
        let api_docs = ws.root.join("docs").join("api");
        fs::create_dir_all(&web_docs).unwrap();
        fs::create_dir_all(&api_docs).unwrap();
        fs::write(web_docs.join("methodology.md"), "Web methodology").unwrap();
        fs::write(api_docs.join("methodology.md"), "API methodology").unwrap();

        let target = ws.root.join("acme").join("web").join("initial");

        let sections = discover_sections(&ws.root, &target, "web").unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].content.contains("Web methodology"));

        let sections = discover_sections(&ws.root, &target, "api").unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].content.contains("API methodology"));
    }

    #[test]
    fn discover_sections_inheritance_override() {
        let ws = TestWorkspace::new();

        let ws_docs = ws.root.join("docs").join("web");
        fs::create_dir_all(&ws_docs).unwrap();
        fs::write(ws_docs.join("methodology.md"), "Master methodology").unwrap();

        let client = ws.create_client("acme");
        let client_docs = client.join("docs").join("web");
        fs::create_dir_all(&client_docs).unwrap();
        fs::write(client_docs.join("methodology.md"), "Client methodology").unwrap();

        let target = client.join("web").join("initial");
        let sections = discover_sections(&ws.root, &target, "web").unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].content.contains("Client methodology"));
        assert_eq!(sections[0].source_level, "client");
    }

    #[test]
    fn discover_sections_common_directory() {
        let ws = TestWorkspace::new();

        let common_docs = ws.root.join("docs").join("common");
        fs::create_dir_all(&common_docs).unwrap();
        fs::write(common_docs.join("terms.md"), "# Terms\n\nStandard terms").unwrap();

        let target = ws.root.join("acme").join("web").join("initial");
        let sections = discover_sections(&ws.root, &target, "web").unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "terms");
    }

    #[test]
    fn assemble_with_exclude() {
        let ws = TestWorkspace::new();

        let web_docs = ws.root.join("docs").join("web");
        let common_docs = ws.root.join("docs").join("common");
        fs::create_dir_all(&web_docs).unwrap();
        fs::create_dir_all(&common_docs).unwrap();
        fs::write(web_docs.join("methodology.md"), "Methodology").unwrap();
        fs::write(web_docs.join("pricing.md"), "Pricing").unwrap();
        fs::write(common_docs.join("terms.md"), "Terms").unwrap();

        let target = ws.root.join("acme").join("web").join("initial");
        let assembled = assemble_sections(&ws.root, &target, "web", None, Some("pricing")).unwrap();

        assert!(assembled.contains_key("methodology"));
        assert!(!assembled.contains_key("pricing"));
        assert!(assembled.contains_key("terms"));
    }

    #[test]
    fn assemble_with_include_path() {
        let ws = TestWorkspace::new();

        let web_docs = ws.root.join("docs").join("web");
        let api_docs = ws.root.join("docs").join("api");
        fs::create_dir_all(&web_docs).unwrap();
        fs::create_dir_all(&api_docs).unwrap();
        fs::write(web_docs.join("methodology.md"), "Web methodology").unwrap();
        fs::write(api_docs.join("methodology.md"), "API methodology").unwrap();

        let target = ws.root.join("acme").join("web").join("initial");
        let assembled = assemble_sections(
            &ws.root,
            &target,
            "web",
            Some("web/methodology,api/methodology"),
            None,
        )
        .unwrap();

        assert!(assembled.contains_key("web_methodology"));
        assert!(assembled.contains_key("api_methodology"));
        assert!(assembled["web_methodology"].contains("Web methodology"));
        assert!(assembled["api_methodology"].contains("API methodology"));
    }
}
