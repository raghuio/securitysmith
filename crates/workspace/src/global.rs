use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::WorkspaceError;

/// Returns the platform config directory for SecuritySmith.
/// Linux:   ~/.config/securitysmith
/// macOS:   ~/Library/Application Support/securitysmith
/// Windows: %APPDATA%\securitysmith
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("securitysmith"))
}

pub fn config_path() -> Result<Utf8PathBuf, WorkspaceError> {
    let dir = config_dir().ok_or(WorkspaceError::NoConfigDir)?;
    Ok(Utf8PathBuf::from_path_buf(dir.join("config.toml")).unwrap())
}

fn ensure_config_dir() -> Result<PathBuf, WorkspaceError> {
    let dir = config_dir().ok_or(WorkspaceError::NoConfigDir)?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub global: GlobalSection,
    #[serde(default)]
    pub workspaces: Vec<KnownWorkspace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSection {
    #[serde(default)]
    pub default_workspace_root: Option<Utf8PathBuf>,
}

impl Default for GlobalSection {
    fn default() -> Self {
        Self {
            default_workspace_root: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownWorkspace {
    pub name: String,
    pub path: Utf8PathBuf,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            global: GlobalSection::default(),
            workspaces: Vec::new(),
        }
    }
}

impl GlobalConfig {
    pub fn load() -> Result<Self, WorkspaceError> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)?;
        let config: GlobalConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), WorkspaceError> {
        let _ = ensure_config_dir()?;
        let path = config_path()?;
        let toml = toml::to_string_pretty(self)?;
        fs::write(&path, toml)?;
        Ok(())
    }

    pub fn default_root(&self) -> Option<Utf8PathBuf> {
        self.global.default_workspace_root.clone().or_else(|| {
            dirs::home_dir().map(|home| {
                Utf8PathBuf::from_path_buf(home.join("securitysmith")).unwrap()
            })
        })
    }

    pub fn set_default_root(&mut self, path: impl AsRef<Utf8Path>) {
        self.global.default_workspace_root = Some(path.as_ref().to_path_buf());
    }

    pub fn register_workspace(&mut self,
        name: impl Into<String>,
        path: impl AsRef<Utf8Path>,
    ) {
        let name = name.into();
        let path = path.as_ref().to_path_buf();
        self.workspaces.retain(|w| w.name != name && w.path != path);
        self.workspaces.push(KnownWorkspace { name, path });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_config_round_trip() {
        let mut config = GlobalConfig::default();
        config.set_default_root("~/clients");
        config.register_workspace("acme", "/home/user/clients/acme");

        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: GlobalConfig = toml::from_str(&toml).unwrap();

        assert_eq!(
            parsed.default_root(),
            Some(Utf8PathBuf::from("~/clients"))
        );
        assert_eq!(parsed.workspaces.len(), 1);
    }
}
