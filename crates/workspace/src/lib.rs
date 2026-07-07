use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

pub mod client;
pub mod global;

pub use client::Client;
pub use global::GlobalConfig;

pub const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("Not a SecuritySmith workspace")]
    NotAWorkspace,
    #[error("Workspace already exists at {0}")]
    AlreadyExists(Utf8PathBuf),
    #[error("No config directory found on this platform")]
    NoConfigDir,
    #[error("No default workspace root configured. Set one with `sm config set default_workspace_root <path>` or pass a path to `sm new`.")]
    NoDefaultRoot,
    #[error("Not found: {0}")]
    NotFound(Utf8PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceHeader {
    pub name: String,
    pub created: String,
}

impl WorkspaceConfig {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            workspace: WorkspaceHeader {
                name: name.into(),
                created: Utc::now().format("%Y-%m-%d").to_string(),
            },
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self::named("workspace")
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: Utf8PathBuf,
    pub config: WorkspaceConfig,
}

impl Workspace {
    /// Create a new workspace at the given path with an optional name.
    /// If no name is given, the last directory component is used.
    pub fn init(path: impl AsRef<Utf8Path>) -> Result<Self, WorkspaceError> {
        let root = path.as_ref().to_path_buf();
        let name = root.file_name().unwrap_or("workspace").to_string();
        Self::init_named(&root, name)
    }

    /// Create a new workspace with an explicit name.
    pub fn init_named(
        path: impl AsRef<Utf8Path>,
        name: impl Into<String>,
    ) -> Result<Self, WorkspaceError> {
        let root = path.as_ref().to_path_buf();
        let config_path = root.join(CONFIG_FILE);

        if config_path.exists() {
            return Err(WorkspaceError::AlreadyExists(root));
        }

        fs::create_dir_all(&root)?;

        let config = WorkspaceConfig::named(name);
        let toml = toml::to_string_pretty(&config)?;
        fs::write(&config_path, toml)?;

        Ok(Self { root, config })
    }

    /// Find a workspace by walking up from the given directory.
    pub fn find(start: impl AsRef<Utf8Path>) -> Result<Self, WorkspaceError> {
        let mut current = start.as_ref().to_path_buf();
        loop {
            let config_path = current.join(CONFIG_FILE);
            if config_path.exists() {
                let contents = fs::read_to_string(&config_path)?;
                let config: WorkspaceConfig = toml::from_str(&contents)?;
                return Ok(Self { root: current, config });
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Err(WorkspaceError::NotAWorkspace),
            }
        }
    }

    /// Load a workspace from a known root.
    pub fn load(root: impl AsRef<Utf8Path>) -> Result<Self, WorkspaceError> {
        let root = root.as_ref().to_path_buf();
        let config_path = root.join(CONFIG_FILE);
        if !config_path.exists() {
            return Err(WorkspaceError::NotAWorkspace);
        }
        let contents = fs::read_to_string(&config_path)?;
        let config: WorkspaceConfig = toml::from_str(&contents)?;
        Ok(Self { root, config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn workspace_init_creates_config_only() {
        let tmp = TempDir::new().unwrap();
        let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).unwrap();
        let ws = Workspace::init(&path).unwrap();

        assert!(ws.root.join(CONFIG_FILE).exists());
        assert!(!ws.root.join("clients").exists());
    }

    #[test]
    fn workspace_find_walks_up() {
        let tmp = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).unwrap();
        Workspace::init(&root).unwrap();

        let nested = root.join("clients").join("acme").join("projects").join("web");
        fs::create_dir_all(&nested).unwrap();
        let found = Workspace::find(&nested).unwrap();

        assert_eq!(found.root, root);
    }
}
