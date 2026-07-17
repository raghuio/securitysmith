#![allow(clippy::collapsible_if)]
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

pub mod check;
pub mod checklists;
pub mod credentials;
pub mod documents;
pub mod entities;
pub mod evidence;
pub mod findings;
pub mod global;
pub mod import;
pub mod notes;
pub mod render;
pub mod requirements;
pub mod scope;
pub mod search;
pub mod sections;
pub mod stats;
pub mod templates;
pub mod typst_engine;

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
    #[error("Frontmatter error: {0}")]
    Frontmatter(#[from] ss_frontmatter::FrontmatterError),
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
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid date: {0}")]
    InvalidDate(String),
    #[error("Invalid status or severity: {0}")]
    InvalidStatusSeverity(String),
    #[error("Symlink escapes the workspace: {0}")]
    SymlinkEscape(Utf8PathBuf),
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
    pub defaults: entities::DefaultsSection,
    pub require_credentials_ready: bool,
}

impl WorkspaceConfig {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            workspace: WorkspaceHeader {
                version: 1,
                name: name.into(),
                created: Utc::now().format("%Y-%m-%d").to_string(),
                defaults: entities::DefaultsSection::default(),
                require_credentials_ready: false,
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

        // Use built-in workspace config template, fill in dynamic fields.
        // No workspace-level override possible here — the workspace doesn't exist yet.
        let template =
            entities::builtin_config_template("workspace").ok_or(WorkspaceError::NotAWorkspace)?;
        let mut config: WorkspaceConfig = toml::from_str(template)?;
        config.workspace.name = name.into();
        config.workspace.created = Utc::now().format("%Y-%m-%d").to_string();

        let toml = toml::to_string_pretty(&config)?;
        atomic_write(&config_path, toml.as_bytes())?;

        Ok(Self { root, config })
    }

    /// Find a workspace by walking up from the given directory.
    pub fn find(start: impl AsRef<Utf8Path>) -> Result<Self, WorkspaceError> {
        let root = Self::find_root(start)?;
        Self::load(&root)
    }

    /// Find workspace root by walking up from the given directory.
    /// Returns the path only — does not parse config.toml.
    pub fn find_root(start: impl AsRef<Utf8Path>) -> Result<Utf8PathBuf, WorkspaceError> {
        let mut current = start.as_ref().to_path_buf();
        loop {
            let config_path = current.join(CONFIG_FILE);
            if config_path.exists() {
                return Ok(current);
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
        let root = Self::resolve_root(explicit)?;
        Self::load(&root)
    }

    /// Resolve workspace root path without parsing config.
    /// Same priority as `resolve()` but returns path only.
    /// Used by the CLI to scan for migration before loading.
    pub fn resolve_root(explicit: Option<&str>) -> Result<Utf8PathBuf, WorkspaceError> {
        // 1. Explicit -w flag
        if let Some(spec) = explicit {
            // Try as a registered workspace name first
            let global = global::GlobalConfig::load()?;
            if let Some(ws) = global.find_workspace(spec) {
                return Ok(ws.path.clone());
            }
            // Try as a path
            let path = expand_tilde(spec);
            if path.join(CONFIG_FILE).exists() {
                return Ok(path);
            }
            return Err(WorkspaceError::NotAWorkspace);
        }

        // 2. Walk up from current directory
        let cwd = current_dir()?;
        if let Ok(root) = Self::find_root(&cwd) {
            return Ok(root);
        }

        // 3. Fall back to default workspace.
        // If the default workspace doesn't have config.toml yet, auto-create it.
        // This makes `sm new acme` work from anywhere — the default workspace
        // is initialized on first use (FR-3, FR-21).
        let global = global::GlobalConfig::load()?;
        if let Some(default) = global.default_workspace() {
            if default.join(CONFIG_FILE).exists() {
                return Ok(default);
            }
            // Auto-create the default workspace if the directory exists or can be created
            if default.exists() || std::fs::create_dir_all(&default).is_ok() {
                let ws = Self::init(&default)?;
                let mut global = global;
                let name = ws.config.workspace.name.clone();
                global.register_workspace(&name, &ws.root);
                global.save()?;
                return Ok(ws.root);
            }
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

/// Verify that a path does not escape the workspace via symlinks.
///
/// If the path exists, canonicalizes it and checks it's inside the canonical
/// workspace root. If the path doesn't exist yet (creation), checks the
/// existing parent components for symlinks.
///
/// Returns Ok if the path is safe, Err(SymlinkEscape) if it escapes.
pub fn check_symlink_escape(ws_root: &Utf8Path, path: &Utf8Path) -> Result<(), WorkspaceError> {
    let canonical_root = fs::canonicalize(ws_root.as_std_path())?;

    // If the full path exists, canonicalize and check.
    if path.exists() {
        let canonical = fs::canonicalize(path.as_std_path())?;
        if !canonical.starts_with(&canonical_root) {
            return Err(WorkspaceError::SymlinkEscape(path.to_path_buf()));
        }
        return Ok(());
    }

    // Path doesn't exist yet (creation). Walk up to the deepest existing
    // ancestor and check from there.
    let mut existing = path.to_path_buf();
    while !existing.exists() {
        match existing.parent() {
            Some(p) if p != existing => existing = p.to_path_buf(),
            _ => return Ok(()), // reached root or no parent
        }
    }

    let canonical_existing = fs::canonicalize(existing.as_std_path())?;
    if !canonical_existing.starts_with(&canonical_root) {
        return Err(WorkspaceError::SymlinkEscape(path.to_path_buf()));
    }

    Ok(())
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

/// Get the current date as a UTC string in YYYY-MM-DD format.
pub fn utc_today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Derive a URL-safe slug from a title.
/// Lowercase, non-alphanumeric → underscore, trimmed.
pub fn slug_from_title(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    slug.trim_matches('_').to_string()
}

/// Atomic write: write to temp file, then rename.
pub(crate) fn atomic_write(path: &Utf8Path, content: &[u8]) -> Result<(), WorkspaceError> {
    let tmp_path = path.with_extension("tmp");

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

/// Open a file in `$EDITOR`. Falls back to `vi` if `$EDITOR` is not set.
/// Splits `$EDITOR` on whitespace: first token is the program, remaining tokens
/// are arguments, and the file path is the final argument. No shell invocation
/// — safe against injection on file paths. Works on all target platforms.
pub fn spawn_editor(path: &Utf8Path) -> Result<(), WorkspaceError> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        eprintln!("warning: $EDITOR not set, falling back to vi");
        "vi".to_string()
    });

    let mut parts = editor.split_whitespace();
    let prog = parts.next().unwrap_or("vi");
    let args = parts.collect::<Vec<_>>();

    let mut cmd = std::process::Command::new(prog);
    for a in &args {
        cmd.arg(a);
    }
    let status = cmd.arg(path.as_std_path()).status()?;

    if !status.success() {
        return Err(WorkspaceError::Io(std::io::Error::other(format!(
            "{} exited with non-zero status",
            editor
        ))));
    }

    Ok(())
}

/// How an item was removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalMethod {
    /// Moved to OS trash (recoverable).
    Trashed,
    /// Permanently deleted (no trash available on this platform).
    PermanentlyDeleted,
}

/// Move a file or directory to OS trash. Falls back to permanent deletion
/// with a warning when no trash is available (headless servers, OpenBSD
/// without desktop, etc.).
///
/// Returns `RemovalMethod::Trashed` if the item was moved to trash, or
/// `RemovalMethod::PermanentlyDeleted` if the fallback was used.
pub fn trash_or_delete(path: &Utf8Path) -> Result<RemovalMethod, WorkspaceError> {
    match trash::delete(path.as_std_path()) {
        Ok(()) => Ok(RemovalMethod::Trashed),
        Err(e) => {
            eprintln!(
                "warning: No trash available on this system ({}). Deleted permanently.",
                e
            );
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
            Ok(RemovalMethod::PermanentlyDeleted)
        }
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
