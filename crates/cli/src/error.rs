#![allow(dead_code)]
// error.rs — Typed errors with exit-code mapping per spec.
use std::process::exit;

/// Exit codes per spec error table (1-13).
pub mod exit_code {
    pub const NOT_A_WORKSPACE: i32 = 1;
    pub const ENTITY_NOT_FOUND: i32 = 2;
    pub const DUPLICATE_ENTITY: i32 = 3;
    pub const INVALID_TOML: i32 = 4;
    pub const INVALID_FRONTMATTER: i32 = 5;
    pub const MISSING_REQUIRED_FIELD: i32 = 6;
    pub const INVALID_DATE: i32 = 7;
    pub const INVALID_STATUS_SEVERITY: i32 = 8;
    pub const REPORT_FAILED: i32 = 9;
    pub const SOW_FAILED: i32 = 10;
    pub const FORCE_FLAG_MISSING: i32 = 11;
    pub const INVALID_NAME_FORMAT: i32 = 12;
    pub const RESERVED_NAME: i32 = 13;
}

/// Print error to stderr and exit with the appropriate code.
pub fn fail(message: &str, code: i32) -> ! {
    eprintln!("error: {}", message);
    exit(code);
}

/// Map a WorkspaceError to the appropriate exit code and message.
pub fn workspace_error(e: &ss_workspace::WorkspaceError) -> (String, i32) {
    use ss_workspace::WorkspaceError::*;
    match e {
        NotAWorkspace => (
            "No SecuritySmith workspace found. Run `sm new` to create one, or set a default with `sm config set default_workspace <path>`."
                .to_string(),
            exit_code::NOT_A_WORKSPACE,
        ),
        AlreadyExists(path) => (
            format!("A workspace already exists at {}", path),
            exit_code::DUPLICATE_ENTITY,
        ),
        NoConfigDir => (
            "No config directory found on this platform.".to_string(),
            exit_code::NOT_A_WORKSPACE,
        ),
        NoDefaultWorkspace => (
            "No default workspace configured. Set one with `sm config set default_workspace <path>`."
                .to_string(),
            exit_code::NOT_A_WORKSPACE,
        ),
        NotFound(path) => (
            format!("Not found: {}", path),
            exit_code::ENTITY_NOT_FOUND,
        ),
        InvalidName(msg) => (msg.clone(), exit_code::INVALID_NAME_FORMAT),
        ReservedName(name) => (
            format!("Name `{}` is reserved.", name),
            exit_code::RESERVED_NAME,
        ),
        Io(e) => (format!("IO error: {}", e), 1),
        Serialize(e) => (format!("TOML serialization error: {}", e), exit_code::INVALID_TOML),
        Deserialize(e) => (format!("TOML deserialization error: {}", e), exit_code::INVALID_TOML),
    }
}
