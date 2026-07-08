//! Entity model for clients, projects, and engagements.
//!
//! All entities are directories with config.toml. No wrapper directories.
//! Depth determines type: depth 1 = client, depth 2 = project, depth 3 = engagement.

use camino::Utf8PathBuf;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

use crate::{validate_name, Workspace, WorkspaceError};

/// Entity types identifiable by config.toml section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Client,
    Project,
    Engagement,
}

impl EntityType {
    pub fn section_name(&self) -> &'static str {
        match self {
            EntityType::Client => "client",
            EntityType::Project => "project",
            EntityType::Engagement => "engagement",
        }
    }

    pub fn from_depth(depth: usize) -> Option<Self> {
        match depth {
            1 => Some(EntityType::Client),
            2 => Some(EntityType::Project),
            3 => Some(EntityType::Engagement),
            _ => None,
        }
    }
}

/// Client config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub client: ClientSection,
    #[serde(default)]
    pub client_id: Option<ClientIdSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSection {
    pub status: String,
    pub priority: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdSection {
    pub prefix: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        let now = Utc::now().format("%Y-%m-%d").to_string();
        Self {
            client: ClientSection {
                status: "active".to_string(),
                priority: "medium".to_string(),
                email: None,
                phone: None,
                tags: Vec::new(),
                notes: None,
                created: now.clone(),
                updated: now,
            },
            client_id: Some(ClientIdSection {
                prefix: "TEST".to_string(),
            }),
        }
    }
}

/// Project config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    #[serde(default)]
    pub project_id: Option<ProjectIdSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub abbreviation: String,
    pub status: String,
    pub priority: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIdSection {
    pub sequence: u32,
    pub padding: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectSection {
                abbreviation: "GEN".to_string(),
                status: "active".to_string(),
                priority: "medium".to_string(),
                start_date: None,
                end_date: None,
                tags: Vec::new(),
                notes: None,
            },
            project_id: Some(ProjectIdSection {
                sequence: 1,
                padding: 3,
            }),
        }
    }
}

/// Engagement config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementConfig {
    pub engagement: EngagementSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementSection {
    pub r#type: String,
    pub status: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl Default for EngagementConfig {
    fn default() -> Self {
        Self {
            engagement: EngagementSection {
                r#type: "assessment".to_string(),
                status: "in_progress".to_string(),
                start_date: None,
                end_date: None,
                target: None,
                notes: None,
            },
        }
    }
}

/// Resolve a path relative to a workspace into an entity directory.
/// Returns (entity_path, depth) where depth is 1=client, 2=project, 3=engagement.
pub fn resolve_path(ws: &Workspace, path: &str) -> Result<(Utf8PathBuf, usize), WorkspaceError> {
    let segments: Vec<&str> = path.split('/').collect();
    let depth = segments.len();

    if depth == 0 || depth > 3 {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(path)));
    }

    // Validate each segment
    for seg in &segments {
        if !seg.is_empty() {
            validate_name(seg)?;
        }
    }

    let mut current = ws.root.clone();
    for seg in &segments {
        current = current.join(seg);
    }

    Ok((current, depth))
}

/// Create a client directory with config.toml.
pub fn create_client(ws: &Workspace, name: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(name)?;
    let dir = ws.root.join(name);

    if dir.join("config.toml").exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    // Generate prefix from name uppercase
    let prefix = name.to_uppercase();
    let mut config = ClientConfig::default();
    config.client_id = Some(ClientIdSection { prefix });

    let toml = toml::to_string_pretty(&config)
        .map_err(|e| WorkspaceError::Serialize(toml::ser::Error::Custom(e.to_string())))?;
    atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// Create a project directory with config.toml under a client.
pub fn create_project(ws: &Workspace, client: &str, project: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(project)?;
    let client_dir = ws.root.join(client);

    if !client_dir.join("config.toml").exists() {
        return Err(WorkspaceError::NotFound(client_dir));
    }

    let dir = client_dir.join(project);
    if dir.join("config.toml").exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    // Generate abbreviation from project name uppercase
    let abbr = project
        .split('_')
        .map(|w| w.chars().take(3).collect::<String>())
        .collect::<Vec<_>>()
        .join("")
        .to_uppercase();
    let mut config = ProjectConfig::default();
    config.project.abbreviation = abbr;

    let toml = toml::to_string_pretty(&config)
        .map_err(|e| WorkspaceError::Serialize(toml::ser::Error::Custom(e.to_string())))?;
    atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// Create an engagement directory with config.toml under a project.
pub fn create_engagement(
    ws: &Workspace,
    client: &str,
    project: &str,
    engagement: &str,
) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(engagement)?;
    let project_dir = ws.root.join(client).join(project);

    if !project_dir.join("config.toml").exists() {
        return Err(WorkspaceError::NotFound(project_dir));
    }

    let dir = project_dir.join(engagement);
    if dir.join("config.toml").exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    let config = EngagementConfig::default();
    let toml = toml::to_string_pretty(&config)
        .map_err(|e| WorkspaceError::Serialize(toml::ser::Error::Custom(e.to_string())))?;
    atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// List entities at a given depth under a parent directory.
/// Scans for subdirectories that contain config.toml.
pub fn list_entities(parent: &Utf8PathBuf, depth: usize) -> Result<Vec<String>, WorkspaceError> {
    if !parent.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let config_path = path.join("config.toml");
            if config_path.exists() {
                if let Some(name) = entry.file_name().to_str() {
                    // Verify the config has the right section for this depth
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        let section = EntityType::from_depth(depth)
                            .map(|t| t.section_name())
                            .unwrap_or("");
                        if content.contains(&format!("[{}]", section)) {
                            names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Show config.toml content for an entity.
pub fn show_config(path: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let config_path = path.join("config.toml");
    if !config_path.exists() {
        return Err(WorkspaceError::NotFound(path.clone()));
    }
    Ok(fs::read_to_string(&config_path)?)
}

/// Open config.toml in $EDITOR (fallback to vi).
pub fn edit_config(path: &Utf8PathBuf) -> Result<(), WorkspaceError> {
    let config_path = path.join("config.toml");
    if !config_path.exists() {
        return Err(WorkspaceError::NotFound(path.clone()));
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        eprintln!("warning: $EDITOR not set, falling back to vi");
        "vi".to_string()
    });

    let status = std::process::Command::new(&editor)
        .arg(config_path.as_std_path())
        .status()?;

    if !status.success() {
        return Err(WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("{} exited with non-zero status", editor),
        )));
    }

    Ok(())
}

/// Remove an entity directory. Requires --force (checked by caller).
pub fn remove_entity(path: &Utf8PathBuf) -> Result<(), WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::NotFound(path.clone()));
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

/// Increment the project sequence counter and return the new ID.
pub fn increment_sequence(project_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let config_path = project_dir.join("config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut config: ProjectConfig = toml::from_str(&content)?;

    let seq = config
        .project_id
        .as_mut()
        .ok_or(WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "project config missing [project.id] section",
        )))?;

    let padding = seq.padding as usize;
    let id = format!("{:0>padding$}", seq.sequence, padding = padding);
    seq.sequence += 1;

    // Write back
    let toml = toml::to_string_pretty(&config)
        .map_err(|e| WorkspaceError::Serialize(toml::ser::Error::Custom(e.to_string())))?;
    atomic_write(&config_path, toml.as_bytes())?;

    Ok(id)
}

/// Get the client prefix from a client directory.
pub fn get_client_prefix(client_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let content = fs::read_to_string(client_dir.join("config.toml"))?;
    let config: ClientConfig = toml::from_str(&content)?;
    config
        .client_id
        .map(|id| id.prefix)
        .ok_or(WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "client config missing [client.id] section",
        )))
}

/// Get the project abbreviation from a project directory.
pub fn get_project_abbreviation(project_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let content = fs::read_to_string(project_dir.join("config.toml"))?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config.project.abbreviation)
}

/// Atomic write helper.
fn atomic_write(path: &Utf8PathBuf, content: &[u8]) -> Result<(), WorkspaceError> {
    let tmp = path.with_extension("toml.tmp");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&tmp, content)?;
    let file = fs::File::open(&tmp)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn create_and_list_clients() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        create_client(&ws, "foobar").unwrap();

        let clients = list_entities(&ws.root, 1).unwrap();
        assert_eq!(clients, vec!["acme", "foobar"]);
    }

    #[test]
    fn create_project_under_client() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        let proj = create_project(&ws, "acme", "web_app").unwrap();
        assert!(proj.join("config.toml").exists());

        let projects = list_entities(&ws.root.join("acme"), 2).unwrap();
        assert_eq!(projects, vec!["web_app"]);
    }

    #[test]
    fn create_engagement_under_project() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        create_project(&ws, "acme", "web_app").unwrap();
        let eng = create_engagement(&ws, "acme", "web_app", "initial").unwrap();
        assert!(eng.join("config.toml").exists());
    }

    #[test]
    fn duplicate_client_rejected() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        assert!(create_client(&ws, "acme").is_err());
    }

    #[test]
    fn project_without_client_rejected() {
        let ws = TestWorkspace::new();
        assert!(create_project(&ws, "nonexistent", "web_app").is_err());
    }

    #[test]
    fn invalid_name_rejected() {
        let ws = TestWorkspace::new();
        assert!(create_client(&ws, "Acme").is_err());
        assert!(create_client(&ws, "acme corp").is_err());
        assert!(create_client(&ws, "templates").is_err());
    }

    #[test]
    fn show_config_works() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        let content = show_config(&ws.root.join("acme")).unwrap();
        assert!(content.contains("[client]"));
    }

    #[test]
    fn remove_entity_works() {
        let ws = TestWorkspace::new();
        let dir = create_client(&ws, "acme").unwrap();
        assert!(dir.exists());
        remove_entity(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn increment_sequence_works() {
        let ws = TestWorkspace::new();
        create_client(&ws, "acme").unwrap();
        create_project(&ws, "acme", "web_app").unwrap();

        let proj_dir = ws.root.join("acme").join("web_app");
        let seq1 = increment_sequence(&proj_dir).unwrap();
        assert_eq!(seq1, "001");
        let seq2 = increment_sequence(&proj_dir).unwrap();
        assert_eq!(seq2, "002");
    }

    #[test]
    fn auto_discovery_finds_manual_client() {
        let ws = TestWorkspace::new();
        // Manually create a client dir with config.toml
        let dir = ws.root.join("manual_client");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.toml"),
            "[client]\nstatus = \"active\"\npriority = \"medium\"\ncreated = \"2026-07-08\"\nupdated = \"2026-07-08\"\n",
        )
        .unwrap();

        // list_entities should find it
        let clients = list_entities(&ws.root, 1).unwrap();
        assert!(clients.contains(&"manual_client".to_string()));
    }
}