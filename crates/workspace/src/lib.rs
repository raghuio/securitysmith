use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

pub mod client;
pub mod global;

#[cfg(test)]
pub mod test_helpers;

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
    #[error(
        "No default workspace configured. Set one with `sm config set default_workspace <path>` or pass a path to `sm new`."
    )]
    NoDefaultWorkspace,
    #[error("Not found: {0}")]
    NotFound(Utf8PathBuf),
    #[error("Invalid name: {0}")]
    InvalidName(String),
    #[error("Reserved name: {0}")]
    ReservedName(String),
}

/// Workspace config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceHeader {
    pub version: u32,
    pub name: String,
    pub created: String,
}

impl WorkspaceConfig {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            workspace: WorkspaceHeader {
                version: 1,
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

/// A discovered workspace on disk.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: Utf8PathBuf,
    pub config: WorkspaceConfig,
}

impl Workspace {
    /// Create a new workspace at the given path.
    /// Writes config.toml with a [workspace] section. Nothing else.
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
        atomic_write(&config_path, toml.as_bytes())?;

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
                return Ok(Self {
                    root: current,
                    config,
                });
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

    /// Resolve which workspace to use.
    /// Priority: explicit > current directory > default workspace
    pub fn resolve(explicit: Option<&str>) -> Result<Self, WorkspaceError> {
        // 1. Explicit -w flag
        if let Some(spec) = explicit {
            // Try as a registered workspace name first
            let global = global::GlobalConfig::load()?;
            if let Some(ws) = global.find_workspace(spec) {
                return Self::load(&ws.path);
            }
            // Try as a path
            let path = expand_tilde(spec);
            if path.join(CONFIG_FILE).exists() {
                return Self::load(&path);
            }
            return Err(WorkspaceError::NotAWorkspace);
        }

        // 2. Walk up from current directory
        let cwd = current_dir()?;
        if let Ok(ws) = Self::find(&cwd) {
            return Ok(ws);
        }

        // 3. Fall back to default workspace
        let global = global::GlobalConfig::load()?;
        if let Some(default) = global.default_workspace()
            && default.join(CONFIG_FILE).exists()
        {
            return Self::load(&default);
        }

        Err(WorkspaceError::NotAWorkspace)
    }
}

/// Validate a snake_case name.
/// Allowed: lowercase letters, digits, underscores. Must not be empty.
/// Reserved: "templates"
pub fn validate_name(name: &str) -> Result<(), WorkspaceError> {
    if name.is_empty() {
        return Err(WorkspaceError::InvalidName(
            "name cannot be empty".to_string(),
        ));
    }

    if name == "templates" {
        return Err(WorkspaceError::ReservedName(name.to_string()));
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(WorkspaceError::InvalidName(format!(
            "Names must be snake_case (a-z, 0-9, underscores). Got: '{}'",
            name
        )));
    }

    Ok(())
}

/// Check if a path looks like an absolute path (starts with / or ~).
pub fn is_absolute_path(s: &str) -> bool {
    s.starts_with('/') || s.starts_with('~')
}

/// Expand ~ to home directory.
pub fn expand_tilde(s: &str) -> Utf8PathBuf {
    if let Some(rest) = s.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        let expanded = home.join(rest);
        return Utf8PathBuf::from_path_buf(expanded).unwrap_or_else(|_| Utf8PathBuf::from(s));
    }
    Utf8PathBuf::from(s)
}

/// Get the current directory as a Utf8PathBuf.
pub fn current_dir() -> Result<Utf8PathBuf, WorkspaceError> {
    let cwd = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(cwd).map_err(|p| {
        WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("current directory is not valid UTF-8: {:?}", p),
        ))
    })
}

/// Atomic write: write to temp file, then rename.
fn atomic_write(path: &Utf8Path, content: &[u8]) -> Result<(), WorkspaceError> {
    let tmp_path = path.with_extension("toml.tmp");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&tmp_path, content)?;

    // Sync the file
    let file = fs::File::open(&tmp_path)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;

    Ok(())
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

        let nested = root.join("acme").join("web_app").join("initial");
        fs::create_dir_all(&nested).unwrap();
        let found = Workspace::find(&nested).unwrap();

        assert_eq!(found.root, root);
    }

    #[test]
    fn validate_name_accepts_snake_case() {
        assert!(validate_name("acme").is_ok());
        assert!(validate_name("web_app_2026").is_ok());
        assert!(validate_name("abc123").is_ok());
    }

    #[test]
    fn validate_name_rejects_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("Acme").is_err());
        assert!(validate_name("acme-corp").is_err());
        assert!(validate_name("acme corp").is_err());
        assert!(validate_name("templates").is_err());
    }

    #[test]
    fn is_absolute_path_detects() {
        assert!(is_absolute_path("/home/user/test"));
        assert!(is_absolute_path("~/securitysmith"));
        assert!(!is_absolute_path("acme"));
        assert!(!is_absolute_path("acme/web_app"));
    }

    #[test]
    fn atomic_write_works() {
        let tmp = TempDir::new().unwrap();
        let path = Utf8PathBuf::from_path_buf(tmp.path().join("test.toml")).unwrap();
        atomic_write(&path, b"version = 1\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "version = 1\n");
    }
}
