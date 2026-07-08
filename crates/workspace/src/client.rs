use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::Workspace;
use crate::WorkspaceError;

pub const CLIENT_FILE: &str = "client-config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub short_name: String,
    pub display_name: String,
    pub registered_name: Option<String>,
    pub country: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub business_tier: Option<String>,
    pub priority: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
    pub notes: Option<String>,
}

impl Client {
    pub fn new(short_name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            short_name: short_name.into(),
            display_name: display_name.into(),
            registered_name: None,
            country: None,
            address: None,
            email: None,
            phone: None,
            business_tier: None,
            priority: None,
            status: "active".into(),
            tags: Vec::new(),
            notes: None,
        }
    }

    pub fn dir(&self, workspace: &Workspace) -> Utf8PathBuf {
        workspace.root.join("clients").join(&self.short_name)
    }

    pub fn path(&self, workspace: &Workspace) -> Utf8PathBuf {
        self.dir(workspace).join(CLIENT_FILE)
    }

    pub fn create(&self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        let dir = self.dir(workspace);
        if dir.join(CLIENT_FILE).exists() {
            return Err(WorkspaceError::AlreadyExists(dir));
        }
        fs::create_dir_all(&dir)?;
        let toml = toml::to_string_pretty(self)?;
        fs::write(dir.join(CLIENT_FILE), toml)?;
        Ok(())
    }

    /// Remove a client directory and all its contents.
    pub fn remove(workspace: &Workspace, short_name: &str) -> Result<(), WorkspaceError> {
        let dir = workspace.root.join("clients").join(short_name);
        if !dir.exists() {
            return Err(WorkspaceError::NotFound(dir));
        }
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// Rename a client directory and update the short_name in client.toml.
    pub fn rename(
        workspace: &Workspace,
        old_short: &str,
        new_short: impl Into<String>,
    ) -> Result<(), WorkspaceError> {
        let new_short = new_short.into();
        if new_short.is_empty() {
            return Err(WorkspaceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "new short name cannot be empty",
            )));
        }

        let old_dir = workspace.root.join("clients").join(old_short);
        let new_dir = workspace.root.join("clients").join(&new_short);

        if !old_dir.exists() {
            return Err(WorkspaceError::NotFound(old_dir));
        }
        if new_dir.exists() {
            return Err(WorkspaceError::AlreadyExists(new_dir));
        }

        let mut client = Self::load(workspace, old_short)?;
        client.short_name = new_short.clone();

        fs::rename(&old_dir, &new_dir)?;

        let toml = toml::to_string_pretty(&client)?;
        fs::write(new_dir.join(CLIENT_FILE), toml)?;

        Ok(())
    }

    pub fn load(workspace: &Workspace, short_name: &str) -> Result<Self, WorkspaceError> {
        let path = workspace
            .root
            .join("clients")
            .join(short_name)
            .join(CLIENT_FILE);
        if !path.exists() {
            return Err(WorkspaceError::NotFound(path));
        }
        let contents = fs::read_to_string(&path)?;
        let client: Client = toml::from_str(&contents)?;
        Ok(client)
    }

    pub fn list(workspace: &Workspace) -> Result<Vec<Self>, WorkspaceError> {
        let clients_dir = workspace.root.join("clients");
        if !clients_dir.exists() {
            return Ok(Vec::new());
        }

        let mut clients = Vec::new();
        for entry in fs::read_dir(&clients_dir)? {
            let entry = entry?;
            let path = entry.path();
            let client_file = path.join(CLIENT_FILE);
            if client_file.exists() {
                let contents = fs::read_to_string(&client_file)?;
                if let Ok(client) = toml::from_str::<Client>(&contents) {
                    clients.push(client);
                }
            }
        }
        clients.sort_by(|a, b| a.short_name.cmp(&b.short_name));
        Ok(clients)
    }

    /// Move a client to another workspace.
    /// Attempts a fast rename first; falls back to copy-and-remove if the
    /// workspaces live on different filesystems.
    pub fn move_to_workspace(
        workspace: &Workspace,
        short_name: &str,
        target_workspace: &Workspace,
    ) -> Result<(), WorkspaceError> {
        let source_dir = workspace.root.join("clients").join(short_name);
        let target_dir = target_workspace.root.join("clients").join(short_name);

        if !source_dir.exists() {
            return Err(WorkspaceError::NotFound(source_dir));
        }
        if target_dir.exists() {
            return Err(WorkspaceError::AlreadyExists(target_dir));
        }

        fs::create_dir_all(target_dir.parent().unwrap())?;

        if fs::rename(&source_dir, &target_dir).is_err() {
            Self::copy_dir_all(source_dir.as_std_path(), target_dir.as_std_path())?;
            fs::remove_dir_all(&source_dir)?;
        }

        Ok(())
    }

    fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), WorkspaceError> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if ty.is_dir() {
                Self::copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }
}
