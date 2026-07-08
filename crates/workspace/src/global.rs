use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::WorkspaceError;

/// Returns the platform config directory for SecuritySmith.
pub fn config_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("securitysmith"))
}

pub fn config_path() -> Result<Utf8PathBuf, WorkspaceError> {
    let dir = config_dir().ok_or(WorkspaceError::NoConfigDir)?;
    Utf8PathBuf::from_path_buf(dir.join("config.toml")).map_err(|p| {
        WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("config path is not valid UTF-8: {:?}", p),
        ))
    })
}

fn ensure_config_dir() -> Result<std::path::PathBuf, WorkspaceError> {
    let dir = config_dir().ok_or(WorkspaceError::NoConfigDir)?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Global configuration stored at ~/.config/securitysmith/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub global: GlobalSection,
    #[serde(default)]
    pub workspaces: Vec<KnownWorkspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSection {
    #[serde(default)]
    pub default_workspace: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnownWorkspace {
    pub name: String,
    pub path: Utf8PathBuf,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let default_path = dirs::home_dir().map(|h| {
            Utf8PathBuf::from_path_buf(h.join("securitysmith"))
                .unwrap_or_else(|_| Utf8PathBuf::from("~/securitysmith"))
        });

        Self {
            global: GlobalSection {
                default_workspace: default_path,
            },
            workspaces: Vec::new(),
        }
    }
}

impl GlobalConfig {
    /// Load the global config. Creates a default one if it doesn't exist.
    pub fn load() -> Result<Self, WorkspaceError> {
        let path = config_path()?;
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        let contents = fs::read_to_string(&path)?;
        let config: GlobalConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save the global config atomically.
    pub fn save(&self) -> Result<(), WorkspaceError> {
        let _ = ensure_config_dir()?;
        let path = config_path()?;
        let toml = toml::to_string_pretty(self)?;

        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, toml.as_bytes())?;
        fs::rename(&tmp, &path)?;

        Ok(())
    }

    /// Get the default workspace path.
    pub fn default_workspace(&self) -> Option<Utf8PathBuf> {
        self.global.default_workspace.clone()
    }

    /// Set the default workspace path.
    pub fn set_default_workspace(&mut self, path: impl AsRef<Utf8Path>) {
        self.global.default_workspace = Some(path.as_ref().to_path_buf());
    }

    /// Register a workspace in the global config.
    /// Removes any existing entry with the same name or path.
    pub fn register_workspace(&mut self, name: impl Into<String>, path: impl AsRef<Utf8Path>) {
        let name = name.into();
        let path = path.as_ref().to_path_buf();
        self.workspaces.retain(|w| w.name != name && w.path != path);
        self.workspaces.push(KnownWorkspace { name, path });
    }

    /// Remove a workspace from the global config.
    pub fn unregister_workspace(&mut self, name: &str) {
        self.workspaces.retain(|w| w.name != name);
    }

    /// Find a workspace by name.
    pub fn find_workspace(&self, name: &str) -> Option<&KnownWorkspace> {
        self.workspaces.iter().find(|w| w.name == name)
    }

    /// Check if a workspace path still exists on disk.
    pub fn verify_workspaces(&self) -> Vec<&KnownWorkspace> {
        self.workspaces
            .iter()
            .filter(|w| !w.path.as_std_path().exists())
            .collect()
    }

    /// Remove stale workspace entries (paths that no longer exist).
    pub fn remove_stale(&mut self) -> usize {
        let before = self.workspaces.len();
        self.workspaces.retain(|w| w.path.as_std_path().exists());
        before - self.workspaces.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_config_round_trip() {
        let mut config = GlobalConfig::default();
        config.set_default_workspace("~/securitysmith");
        config.register_workspace("test", "/tmp/test");

        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: GlobalConfig = toml::from_str(&toml).unwrap();

        assert_eq!(
            parsed.default_workspace(),
            Some(Utf8PathBuf::from("~/securitysmith"))
        );
        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.workspaces[0].name, "test");
    }

    #[test]
    fn register_and_find_workspace() {
        let mut config = GlobalConfig::default();
        config.register_workspace("2026", "/home/user/securitysmith/2026");
        config.register_workspace("enterprise", "/home/user/projects/enterprise");

        assert!(config.find_workspace("2026").is_some());
        assert!(config.find_workspace("enterprise").is_some());
        assert!(config.find_workspace("nonexistent").is_none());
    }

    #[test]
    fn unregister_workspace() {
        let mut config = GlobalConfig::default();
        config.register_workspace("2026", "/tmp/2026");
        config.register_workspace("2027", "/tmp/2027");

        config.unregister_workspace("2026");
        assert!(config.find_workspace("2026").is_none());
        assert!(config.find_workspace("2027").is_some());
    }
}
