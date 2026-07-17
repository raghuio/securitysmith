//! Entity model for clients, projects, and engagements.
//!
//! All entities are directories with config.toml. No wrapper directories.
//! Depth determines type: depth 1 = client, depth 2 = project, depth 3 = engagement.

use camino::Utf8PathBuf;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{Workspace, WorkspaceError, validate_name};

// ── Built-in config templates (Principle 24: no hardcoded config in .rs files) ─
// These .toml files live in crates/workspace/templates/ and are compiled into
// the binary via include_str!. Users can override them at the workspace level
// by creating templates/client.toml, templates/project.toml, etc.
const WORKSPACE_CONFIG_TEMPLATE: &str = include_str!("../templates/workspace.toml");
const CLIENT_CONFIG_TEMPLATE: &str = include_str!("../templates/client.toml");
const PROJECT_CONFIG_TEMPLATE: &str = include_str!("../templates/project.toml");
const ENGAGEMENT_CONFIG_TEMPLATE: &str = include_str!("../templates/engagement.toml");

/// Config template types that users can create/edit in workspace templates/.
pub const CONFIG_TEMPLATE_TYPES: &[&str] = &["workspace", "client", "project", "engagement"];

/// Get a config template by entity type name.
/// Priority: workspace templates/<type>.toml > built-in include_str! > Rust Default impl.
pub fn get_config_template(ws: &Workspace, entity_type: &str) -> Result<String, WorkspaceError> {
    // Check workspace-level override first
    let ws_template_path = ws
        .root
        .join("templates")
        .join(format!("{}.toml", entity_type));
    if ws_template_path.exists() {
        return Ok(fs::read_to_string(&ws_template_path)?);
    }
    // Fall back to built-in
    match entity_type {
        "workspace" => Ok(WORKSPACE_CONFIG_TEMPLATE.to_string()),
        "client" => Ok(CLIENT_CONFIG_TEMPLATE.to_string()),
        "project" => Ok(PROJECT_CONFIG_TEMPLATE.to_string()),
        "engagement" => Ok(ENGAGEMENT_CONFIG_TEMPLATE.to_string()),
        _ => Err(WorkspaceError::NotFound(Utf8PathBuf::from(format!(
            "templates/{}.toml",
            entity_type
        )))),
    }
}

/// Get built-in config template content (no workspace lookup).
/// Used by `sm new templates/<type>.toml` to seed workspace-level templates.
pub fn builtin_config_template(entity_type: &str) -> Option<&'static str> {
    match entity_type {
        "workspace" => Some(WORKSPACE_CONFIG_TEMPLATE),
        "client" => Some(CLIENT_CONFIG_TEMPLATE),
        "project" => Some(PROJECT_CONFIG_TEMPLATE),
        "engagement" => Some(ENGAGEMENT_CONFIG_TEMPLATE),
        _ => None,
    }
}

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
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSection {
    pub status: String,
    pub priority: String,
    pub email: String,
    pub phone: String,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub id: Option<ClientIdSection>,
    pub defaults: DefaultsSection,
    pub report: ReportSettings,
    pub sow: SowSettings,
    pub contacts: Vec<Contact>,
}

/// Shared defaults section for workspace, client, and project configs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsSection {
    pub severity: String,
    pub status: String,
    pub report_format: String,
    pub sow_format: String,
}

/// Report settings in client and project configs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportSettings {
    pub template: String,
    pub include_findings: String,
    pub exclude_status: Vec<String>,
}

/// SOW settings in client and project configs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SowSettings {
    pub template: String,
    pub include_requirements: String,
    pub include_scope: String,
    pub sections: Vec<String>,
}

/// Contact entry in client config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
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
                email: String::new(),
                phone: String::new(),
                tags: Vec::new(),
                created: now.clone(),
                updated: now,
                id: Some(ClientIdSection {
                    prefix: "TEST".to_string(),
                }),
                defaults: DefaultsSection::default(),
                report: ReportSettings::default(),
                sow: SowSettings::default(),
                contacts: Vec::new(),
            },
            version: 1,
        }
    }
}

/// Project config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    pub abbreviation: String,
    pub status: String,
    pub priority: String,
    pub start_date: String,
    pub end_date: String,
    pub tags: Vec<String>,
    pub id: Option<ProjectIdSection>,
    pub defaults: DefaultsSection,
    pub report: ReportSettings,
    pub sow: SowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIdSection {
    pub sequence: u32,
    pub padding: u32,
    pub requirement_sequence: u32,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project: ProjectSection {
                abbreviation: "GEN".to_string(),
                status: "active".to_string(),
                priority: "medium".to_string(),
                start_date: String::new(),
                end_date: String::new(),
                tags: Vec::new(),
                id: Some(ProjectIdSection {
                    sequence: 1,
                    padding: 3,
                    requirement_sequence: 1,
                }),
                defaults: DefaultsSection::default(),
                report: ReportSettings::default(),
                sow: SowSettings::default(),
            },
            version: 1,
        }
    }
}

/// Engagement config.toml content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementConfig {
    pub engagement: EngagementSection,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementSection {
    pub r#type: String,
    pub status: String,
    pub start_date: String,
    pub end_date: String,
    pub credentials_ready: bool,
    pub original_engagement: String,
}

impl Default for EngagementConfig {
    fn default() -> Self {
        Self {
            engagement: EngagementSection {
                r#type: "assessment".to_string(),
                status: "draft".to_string(),
                start_date: String::new(),
                end_date: String::new(),
                credentials_ready: false,
                original_engagement: String::new(),
            },
            version: 1,
        }
    }
}

/// Resolve a path relative to a workspace into an entity directory.
/// Returns (entity_path, depth) where depth is 1=client, 2=project, 3=engagement.
pub fn resolve_path(ws: &Workspace, path: &str) -> Result<(Utf8PathBuf, usize), WorkspaceError> {
    if path.is_empty() || Utf8PathBuf::from(path).is_absolute() {
        return Err(WorkspaceError::InvalidName(
            "Entity paths must be relative to the workspace.".to_string(),
        ));
    }

    let segments: Vec<&str> = path.split('/').collect();
    let depth = segments.len();

    if depth > 3 {
        return Err(WorkspaceError::NotFound(Utf8PathBuf::from(path)));
    }

    for seg in &segments {
        validate_name(seg)?;
    }

    let mut current = ws.root.clone();
    for seg in &segments {
        current = current.join(seg);
    }

    if !current.starts_with(&ws.root) {
        return Err(WorkspaceError::InvalidName(
            "Entity path escapes the workspace.".to_string(),
        ));
    }

    // Check for symlink escape
    crate::check_symlink_escape(&ws.root, &current)?;

    Ok((current, depth))
}

// ── Config migration (Principle 23: no backward compat code) ─────────

/// Config file type for migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigType {
    Workspace,
    Client,
    Project,
    Engagement,
}

/// Result of checking a single config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigCheck {
    /// Parses as current typed struct — no action needed.
    Current,
    /// Fails typed parse but valid TOML — can be migrated.
    NeedsMigration,
    /// Fails even generic TOML parse — corrupt, cannot migrate.
    Corrupt,
}

/// Result of scanning all config files in a workspace.
/// Reports files that need migration (old format) and files that are corrupt.
#[derive(Debug, Clone, Default)]
pub struct ConfigScanReport {
    /// Config files in old format that can be migrated.
    pub needs_migration: Vec<Utf8PathBuf>,
    /// Config files with invalid TOML that cannot be migrated.
    pub corrupt: Vec<Utf8PathBuf>,
}

impl ConfigScanReport {
    pub fn is_clean(&self) -> bool {
        self.needs_migration.is_empty() && self.corrupt.is_empty()
    }
}

/// Recursively fill in missing fields from `default` into `existing`.
/// Existing values are preserved — only missing keys are inserted.
fn merge_toml(existing: &mut toml::Value, default: &toml::Value) {
    if let (toml::Value::Table(existing_tbl), toml::Value::Table(default_tbl)) = (existing, default)
    {
        for (key, val) in default_tbl {
            if !existing_tbl.contains_key(key) {
                existing_tbl.insert(key.clone(), val.clone());
            } else {
                merge_toml(existing_tbl.get_mut(key).unwrap(), val);
            }
        }
    }
}

/// Check if a config file parses as the current typed struct.
/// Returns Current if current format, NeedsMigration if old format,
/// Corrupt if invalid TOML.
fn check_config(
    path: &camino::Utf8Path,
    config_type: ConfigType,
) -> Result<ConfigCheck, WorkspaceError> {
    let content = fs::read_to_string(path)?;

    // Try current typed format.
    let parse_result = match config_type {
        ConfigType::Workspace => toml::from_str::<crate::WorkspaceConfig>(&content).map(|_| ()),
        ConfigType::Client => toml::from_str::<ClientConfig>(&content).map(|_| ()),
        ConfigType::Project => toml::from_str::<ProjectConfig>(&content).map(|_| ()),
        ConfigType::Engagement => toml::from_str::<EngagementConfig>(&content).map(|_| ()),
    };
    if parse_result.is_ok() {
        return Ok(ConfigCheck::Current);
    }

    // Try generic TOML. If this fails, the file is corrupt.
    if toml::from_str::<toml::Value>(&content).is_ok() {
        Ok(ConfigCheck::NeedsMigration)
    } else {
        Ok(ConfigCheck::Corrupt)
    }
}

/// Migrate a single config file to the current format.
/// Fills in missing fields with defaults, preserves existing data.
fn migrate_config_file(
    path: &camino::Utf8Path,
    config_type: ConfigType,
) -> Result<(), WorkspaceError> {
    let content = fs::read_to_string(path)?;
    let mut existing: toml::Value = toml::from_str(&content)?;

    let default = match config_type {
        ConfigType::Workspace => toml::Value::try_from(crate::WorkspaceConfig::default())
            .unwrap_or(toml::Value::Table(toml::value::Table::new())),
        ConfigType::Client => toml::Value::try_from(ClientConfig::default())
            .unwrap_or(toml::Value::Table(toml::value::Table::new())),
        ConfigType::Project => toml::Value::try_from(ProjectConfig::default())
            .unwrap_or(toml::Value::Table(toml::value::Table::new())),
        ConfigType::Engagement => toml::Value::try_from(EngagementConfig::default())
            .unwrap_or(toml::Value::Table(toml::value::Table::new())),
    };
    merge_toml(&mut existing, &default);

    let toml_str = toml::to_string_pretty(&existing)?;
    crate::atomic_write(path, toml_str.as_bytes())?;
    Ok(())
}

/// Scan all config files in a workspace (workspace config + all entity configs).
/// Returns a report of files needing migration and corrupt files.
/// Does NOT modify any files — pure read.
pub fn scan_all_configs(root: &camino::Utf8Path) -> Result<ConfigScanReport, WorkspaceError> {
    let mut report = ConfigScanReport::default();

    // Check workspace config
    let ws_config_path = root.join(crate::CONFIG_FILE);
    if ws_config_path.exists() {
        match check_config(&ws_config_path, ConfigType::Workspace) {
            Ok(ConfigCheck::Current) => {}
            Ok(ConfigCheck::NeedsMigration) => report.needs_migration.push(ws_config_path),
            Ok(ConfigCheck::Corrupt) => report.corrupt.push(ws_config_path),
            Err(_) => report.corrupt.push(ws_config_path),
        }
    }

    // Walk entity configs
    scan_entity_configs(root, root, &mut report)?;
    Ok(report)
}

fn scan_entity_configs(
    dir: &camino::Utf8Path,
    root: &camino::Utf8Path,
    report: &mut ConfigScanReport,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "templates" || name.starts_with('.') {
            continue;
        }

        let path = match Utf8PathBuf::from_path_buf(entry.path()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let config_path = path.join("config.toml");
        if config_path.exists() {
            let depth = path
                .strip_prefix(root)
                .map(|p| p.components().count())
                .unwrap_or(0);
            let config_type = match depth {
                1 => ConfigType::Client,
                2 => ConfigType::Project,
                3 => ConfigType::Engagement,
                _ => {
                    continue;
                }
            };
            match check_config(&config_path, config_type) {
                Ok(ConfigCheck::Current) => {}
                Ok(ConfigCheck::NeedsMigration) => report.needs_migration.push(config_path),
                Ok(ConfigCheck::Corrupt) => report.corrupt.push(config_path),
                Err(_) => report.corrupt.push(config_path),
            }
        }

        scan_entity_configs(&path, root, report)?;
    }
    Ok(())
}

/// Migrate all config files in a workspace to the current format.
/// Only call after user confirmation — this modifies files.
pub fn migrate_all(root: &camino::Utf8Path) -> Result<usize, WorkspaceError> {
    let mut count = 0;

    // Migrate workspace config
    let ws_config_path = root.join(crate::CONFIG_FILE);
    if ws_config_path.exists() {
        if check_config(&ws_config_path, ConfigType::Workspace)? == ConfigCheck::NeedsMigration {
            migrate_config_file(&ws_config_path, ConfigType::Workspace)?;
            count += 1;
        }
    }

    // Migrate entity configs
    count += migrate_entity_configs(root, root)?;
    Ok(count)
}

fn migrate_entity_configs(
    dir: &camino::Utf8Path,
    root: &camino::Utf8Path,
) -> Result<usize, WorkspaceError> {
    let mut count = 0;
    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "templates" || name.starts_with('.') {
            continue;
        }

        let path = match Utf8PathBuf::from_path_buf(entry.path()) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let config_path = path.join("config.toml");
        if config_path.exists() {
            let depth = path
                .strip_prefix(root)
                .map(|p| p.components().count())
                .unwrap_or(0);
            let config_type = match depth {
                1 => ConfigType::Client,
                2 => ConfigType::Project,
                3 => ConfigType::Engagement,
                _ => {
                    continue;
                }
            };
            if check_config(&config_path, config_type)? == ConfigCheck::NeedsMigration {
                migrate_config_file(&config_path, config_type)?;
                count += 1;
            }
        }

        count += migrate_entity_configs(&path, root)?;
    }
    Ok(count)
}

/// Resolve an existing entity and verify its path and config type.
pub fn resolve_existing_entity(
    ws: &Workspace,
    path: &str,
) -> Result<(Utf8PathBuf, EntityType), WorkspaceError> {
    let (entity_path, depth) = resolve_path(ws, path)?;
    let entity_type = EntityType::from_depth(depth)
        .ok_or_else(|| WorkspaceError::NotFound(Utf8PathBuf::from(path)))?;

    let canonical_root = fs::canonicalize(ws.root.as_std_path())?;
    let canonical_entity = fs::canonicalize(entity_path.as_std_path())
        .map_err(|_| WorkspaceError::NotFound(entity_path.clone()))?;
    if !canonical_entity.starts_with(&canonical_root) {
        return Err(WorkspaceError::InvalidName(
            "Entity path escapes the workspace.".to_string(),
        ));
    }

    let config_path = entity_path.join("config.toml");
    let content = fs::read_to_string(&config_path)
        .map_err(|_| WorkspaceError::NotFound(entity_path.clone()))?;
    let config: toml::Value = toml::from_str(&content)?;
    if config.get(entity_type.section_name()).is_none() {
        return Err(WorkspaceError::NotFound(entity_path));
    }

    Ok((entity_path, entity_type))
}

/// Create a client directory with config.toml.
pub fn create_client(ws: &Workspace, name: &str) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(name)?;
    let dir = ws.root.join(name);

    if dir.exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    // Read config template (workspace override > built-in), fill dynamic fields
    let template = get_config_template(ws, "client")?;
    let mut config: ClientConfig = toml::from_str(&template)?;
    let now = Utc::now().format("%Y-%m-%d").to_string();
    config.client.created = now.clone();
    config.client.updated = now;
    config.client.id = Some(ClientIdSection {
        prefix: name.to_uppercase(),
    });

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// Create a project directory with config.toml under a client.
pub fn create_project(
    ws: &Workspace,
    client: &str,
    project: &str,
) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(client)?;
    validate_name(project)?;
    let (client_dir, entity_type) = resolve_existing_entity(ws, client)?;
    if entity_type != EntityType::Client {
        return Err(WorkspaceError::NotFound(client_dir));
    }

    let dir = client_dir.join(project);
    if dir.exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    // Generate abbreviation from first word of project name (first 3 chars, uppercase)
    let abbr = project
        .split('_')
        .next()
        .map(|w| w.chars().take(3).collect::<String>())
        .unwrap_or_else(|| "GEN".to_string())
        .to_uppercase();
    if abbr.is_empty() {
        return Err(WorkspaceError::InvalidName(
            "Could not generate abbreviation from project name".to_string(),
        ));
    }

    // Read config template (workspace override > built-in), fill dynamic fields
    let template = get_config_template(ws, "project")?;
    let mut config: ProjectConfig = toml::from_str(&template)?;
    config.project.abbreviation = abbr;

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// Create an engagement directory with config.toml under a project.
pub fn create_engagement(
    ws: &Workspace,
    client: &str,
    project: &str,
    engagement: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Utf8PathBuf, WorkspaceError> {
    validate_name(client)?;
    validate_name(project)?;
    validate_name(engagement)?;
    let project_path = format!("{}/{}", client, project);
    let (project_dir, entity_type) = resolve_existing_entity(ws, &project_path)?;
    if entity_type != EntityType::Project {
        return Err(WorkspaceError::NotFound(project_dir));
    }

    let dir = project_dir.join(engagement);
    if dir.exists() {
        return Err(WorkspaceError::AlreadyExists(dir));
    }

    fs::create_dir_all(&dir)?;

    // Read config template (workspace override > built-in), fill dynamic fields
    let template = get_config_template(ws, "engagement")?;
    let mut config: EngagementConfig = toml::from_str(&template)?;
    config.engagement.start_date = start_date.unwrap_or_default().to_string();
    config.engagement.end_date = end_date.unwrap_or_default().to_string();

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&dir.join("config.toml"), toml.as_bytes())?;

    Ok(dir)
}

/// Valid engagement status values.
pub const VALID_ENGAGEMENT_STATUSES: &[&str] = &[
    "draft",
    "planned",
    "in_progress",
    "paused",
    "completed",
    "closed",
];

/// Check if a status value is valid.
pub fn is_valid_engagement_status(status: &str) -> bool {
    VALID_ENGAGEMENT_STATUSES.contains(&status)
}

/// Update engagement status in config.toml.
/// Enforces credential gate if workspace requires it.
/// Create a retest engagement linked to an original engagement.
/// Copies fixed/risk_accepted findings with retest fields.
pub fn create_retest_engagement(
    ws: &Workspace,
    client: &str,
    project: &str,
    retest_name: &str,
    original_name: &str,
) -> Result<usize, WorkspaceError> {
    validate_name(retest_name)?;
    let project_path = format!("{}/{}", client, project);
    let (project_dir, entity_type) = resolve_existing_entity(ws, &project_path)?;
    if entity_type != EntityType::Project {
        return Err(WorkspaceError::NotFound(project_dir));
    }

    // Verify original engagement exists
    let original_path = format!("{}/{}", project_path, original_name);
    let (original_dir, orig_type) = resolve_existing_entity(ws, &original_path)?;
    if orig_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(original_dir));
    }

    // Create retest engagement
    let retest_dir = project_dir.join(retest_name);
    if retest_dir.exists() {
        return Err(WorkspaceError::AlreadyExists(retest_dir));
    }
    fs::create_dir_all(&retest_dir)?;

    let mut config = EngagementConfig::default();
    config.engagement.r#type = "retest".to_string();
    config.engagement.original_engagement = original_name.to_string();
    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&retest_dir.join("config.toml"), toml.as_bytes())?;

    // Copy findings from original that are fixed or risk_accepted
    let orig_findings_dir = original_dir.join("findings");
    let retest_findings_dir = retest_dir.join("findings");
    let mut copied = 0;

    if orig_findings_dir.exists() {
        fs::create_dir_all(&retest_findings_dir)?;

        for entry in fs::read_dir(orig_findings_dir.as_std_path())? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if !filename_str.ends_with(".md") {
                continue;
            }

            let content = fs::read_to_string(entry.path())?;
            let parsed = ss_frontmatter::parse(&content)?;

            // Only copy fixed or risk_accepted findings
            let status = parsed
                .frontmatter
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if status != "fixed" && status != "risk_accepted" {
                continue;
            }

            // Get original finding ID
            let orig_id = parsed
                .frontmatter
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Generate new ID
            let new_id = increment_sequence(&project_dir)?;
            let severity = parsed
                .frontmatter
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string();

            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

            // Build new finding content with retest fields
            let new_content = format!(
                "---\nid: \"{}\"\nstatus: \"open\"\nseverity: \"{}\"\ncreated: \"{}\"\nupdated: \"{}\"\noriginal_finding_id: \"{}\"\nretest_result: \"not_tested\"\n---\n\n{}",
                new_id, severity, today, today, orig_id, parsed.body
            );

            let slug: String = filename_str
                .trim_end_matches(".md")
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let new_filename = format!("{}_{}.md", new_id.to_lowercase(), slug);
            crate::atomic_write(
                &retest_findings_dir.join(&new_filename),
                new_content.as_bytes(),
            )?;
            copied += 1;
        }
    }

    Ok(copied)
}

pub fn update_engagement_status(
    ws: &Workspace,
    path: &str,
    new_status: &str,
) -> Result<(), WorkspaceError> {
    if !is_valid_engagement_status(new_status) {
        return Err(WorkspaceError::InvalidStatusSeverity(format!(
            "'{}' is not a valid engagement status. Use: {}.",
            new_status,
            VALID_ENGAGEMENT_STATUSES.join(", ")
        )));
    }

    let (eng_dir, entity_type) = resolve_existing_entity(ws, path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(eng_dir));
    }

    let config_path = eng_dir.join("config.toml");
    let content = std::fs::read_to_string(config_path.as_std_path())?;
    let mut config: EngagementConfig = toml::from_str(&content)?;

    // Credential gate: moving to in_progress requires credentials_ready
    if new_status == "in_progress" {
        if ws.config.workspace.require_credentials_ready && !config.engagement.credentials_ready {
            return Err(WorkspaceError::MissingField(format!(
                "Cannot start engagement: credentials not marked ready. Use `sm engagement {} --credentials-ready` first.",
                path
            )));
        }
    }

    config.engagement.status = new_status.to_string();
    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&config_path, toml.as_bytes())?;
    Ok(())
}

/// Update engagement start or end date.
pub fn update_engagement_date(
    ws: &Workspace,
    path: &str,
    field: &str,
    date: &str,
) -> Result<(), WorkspaceError> {
    // Validate date format YYYY-MM-DD
    if !date.is_empty() && chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return Err(WorkspaceError::InvalidDate(date.to_string()));
    }

    let (eng_dir, entity_type) = resolve_existing_entity(ws, path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(eng_dir));
    }

    let config_path = eng_dir.join("config.toml");
    let content = std::fs::read_to_string(config_path.as_std_path())?;
    let mut config: EngagementConfig = toml::from_str(&content)?;

    match field {
        "start_date" => config.engagement.start_date = date.to_string(),
        "end_date" => config.engagement.end_date = date.to_string(),
        _ => {
            return Err(WorkspaceError::InvalidName(format!(
                "Unknown date field: {}",
                field
            )));
        }
    }

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&config_path, toml.as_bytes())?;
    Ok(())
}

/// Toggle the credentials_ready field on an engagement.
pub fn toggle_credentials_ready(ws: &Workspace, path: &str) -> Result<bool, WorkspaceError> {
    let (eng_dir, entity_type) = resolve_existing_entity(ws, path)?;
    if entity_type != EntityType::Engagement {
        return Err(WorkspaceError::NotFound(eng_dir));
    }

    let config_path = eng_dir.join("config.toml");
    let content = std::fs::read_to_string(config_path.as_std_path())?;
    let mut config: EngagementConfig = toml::from_str(&content)?;

    config.engagement.credentials_ready = !config.engagement.credentials_ready;
    let new_value = config.engagement.credentials_ready;

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&config_path, toml.as_bytes())?;
    Ok(new_value)
}
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

/// Engagement summary for status display.
#[derive(Debug, Clone)]
pub struct EngagementSummary {
    pub client: String,
    pub project: String,
    pub name: String,
    pub status: String,
    pub start_date: String,
    pub end_date: String,
}

/// List all engagements under a client, grouped by project.
/// Returns summaries with status and dates for display.
pub fn list_client_engagements(
    ws: &Workspace,
    client: &str,
) -> Result<Vec<EngagementSummary>, WorkspaceError> {
    let (client_dir, entity_type) = resolve_existing_entity(ws, client)?;
    if entity_type != EntityType::Client {
        return Err(WorkspaceError::NotFound(client_dir));
    }

    let mut summaries = Vec::new();

    let projects = list_entities(&client_dir, 2)?;
    for project_name in &projects {
        let project_dir = client_dir.join(project_name);
        let engagements = list_entities(&project_dir, 3)?;
        for eng_name in &engagements {
            let eng_dir = project_dir.join(eng_name);
            let config_path = eng_dir.join("config.toml");
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<EngagementConfig>(&content) {
                    summaries.push(EngagementSummary {
                        client: client.to_string(),
                        project: project_name.clone(),
                        name: eng_name.clone(),
                        status: config.engagement.status,
                        start_date: config.engagement.start_date,
                        end_date: config.engagement.end_date,
                    });
                }
            }
        }
    }

    // Sort by project then engagement name for stable output
    summaries.sort_by(|a, b| {
        a.client
            .cmp(&b.client)
            .then_with(|| a.project.cmp(&b.project))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(summaries)
}

/// Show config.toml content for an entity.
pub fn show_config(ws: &Workspace, path: &str) -> Result<String, WorkspaceError> {
    let (entity_path, _) = resolve_existing_entity(ws, path)?;
    Ok(fs::read_to_string(entity_path.join("config.toml"))?)
}

/// Open config.toml in $EDITOR (fallback to vi).
pub fn edit_config(ws: &Workspace, path: &str) -> Result<(), WorkspaceError> {
    let (entity_path, _) = resolve_existing_entity(ws, path)?;
    let config_path = entity_path.join("config.toml");
    crate::spawn_editor(&config_path)
}

/// Remove an entity directory. Moves to OS trash; falls back to permanent deletion
/// with warning when no trash is available.
pub fn remove_entity(ws: &Workspace, path: &str) -> Result<crate::RemovalMethod, WorkspaceError> {
    let (entity_path, _) = resolve_existing_entity(ws, path)?;
    crate::trash_or_delete(&entity_path)
}

/// Increment the project sequence counter and return the new ID.
pub fn increment_sequence(project_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let config_path = project_dir.join("config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut config: ProjectConfig = toml::from_str(&content)?;

    let seq = config
        .project
        .id
        .as_mut()
        .ok_or(WorkspaceError::MissingField(
            "project config missing [project.id] section".to_string(),
        ))?;
    let padding = seq.padding as usize;
    let id = format!("{:0>padding$}", seq.sequence, padding = padding);
    seq.sequence += 1;

    // Write back
    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&config_path, toml.as_bytes())?;

    Ok(id)
}

/// Get the client prefix from a client directory.
pub fn get_client_prefix(client_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let content = fs::read_to_string(client_dir.join("config.toml"))?;
    let config: ClientConfig = toml::from_str(&content)?;
    config
        .client
        .id
        .map(|id| id.prefix)
        .ok_or(WorkspaceError::MissingField(
            "client config missing [client.id] section".to_string(),
        ))
}

/// Get the project abbreviation from a project directory.
pub fn get_project_abbreviation(project_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let content = fs::read_to_string(project_dir.join("config.toml"))?;
    let config: ProjectConfig = toml::from_str(&content)?;
    Ok(config.project.abbreviation)
}

/// Effective defaults after walking the inheritance chain.
#[derive(Debug, Clone)]
pub struct EffectiveDefaults {
    pub severity: String,
    pub status: String,
    pub report_format: String,
    pub sow_format: String,
}

impl Default for EffectiveDefaults {
    fn default() -> Self {
        Self {
            severity: "medium".to_string(),
            status: "open".to_string(),
            report_format: "markdown".to_string(),
            sow_format: "markdown".to_string(),
        }
    }
}

/// Walk the config defaults inheritance chain: project > client > workspace > built-in.
pub fn get_effective_defaults(
    ws: &crate::Workspace,
    client_name: &str,
    project_name: Option<&str>,
) -> EffectiveDefaults {
    let mut ef = EffectiveDefaults::default();

    // Workspace defaults (lowest priority)
    let d = &ws.config.workspace.defaults;
    if !d.severity.is_empty() {
        ef.severity = d.severity.clone();
    }
    if !d.status.is_empty() {
        ef.status = d.status.clone();
    }
    if !d.report_format.is_empty() {
        ef.report_format = d.report_format.clone();
    }
    if !d.sow_format.is_empty() {
        ef.sow_format = d.sow_format.clone();
    }

    // Client defaults (middle priority)
    let client_dir = ws.root.join(client_name);
    if let Ok(content) = fs::read_to_string(client_dir.join("config.toml")) {
        if let Ok(config) = toml::from_str::<ClientConfig>(&content) {
            let d = &config.client.defaults;
            if !d.severity.is_empty() {
                ef.severity = d.severity.clone();
            }
            if !d.status.is_empty() {
                ef.status = d.status.clone();
            }
            if !d.report_format.is_empty() {
                ef.report_format = d.report_format.clone();
            }
            if !d.sow_format.is_empty() {
                ef.sow_format = d.sow_format.clone();
            }
        }
    }

    // Project defaults (highest priority)
    if let Some(pname) = project_name {
        let project_dir = client_dir.join(pname);
        if let Ok(content) = fs::read_to_string(project_dir.join("config.toml")) {
            if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
                let d = &config.project.defaults;
                if !d.severity.is_empty() {
                    ef.severity = d.severity.clone();
                }
                if !d.status.is_empty() {
                    ef.status = d.status.clone();
                }
                if !d.report_format.is_empty() {
                    ef.report_format = d.report_format.clone();
                }
                if !d.sow_format.is_empty() {
                    ef.sow_format = d.sow_format.clone();
                }
            }
        }
    }

    ef
}

/// Get report template name from config inheritance chain.
/// Returns the template name if set (non-empty), following project > client priority.
pub fn get_effective_report_template(
    ws: &crate::Workspace,
    client_name: &str,
    project_name: Option<&str>,
) -> Option<String> {
    if let Some(pname) = project_name {
        let project_dir = ws.root.join(client_name).join(pname);
        if let Ok(content) = fs::read_to_string(project_dir.join("config.toml")) {
            if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
                if !config.project.report.template.is_empty() {
                    return Some(config.project.report.template.clone());
                }
            }
        }
    }
    let client_dir = ws.root.join(client_name);
    if let Ok(content) = fs::read_to_string(client_dir.join("config.toml")) {
        if let Ok(config) = toml::from_str::<ClientConfig>(&content) {
            if !config.client.report.template.is_empty() {
                return Some(config.client.report.template.clone());
            }
        }
    }
    None
}

/// Get SOW template name from config inheritance chain.
/// Returns the template name if set (non-empty), following project > client priority.
pub fn get_effective_sow_template(
    ws: &crate::Workspace,
    client_name: &str,
    project_name: Option<&str>,
) -> Option<String> {
    if let Some(pname) = project_name {
        let project_dir = ws.root.join(client_name).join(pname);
        if let Ok(content) = fs::read_to_string(project_dir.join("config.toml")) {
            if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
                if !config.project.sow.template.is_empty() {
                    return Some(config.project.sow.template.clone());
                }
            }
        }
    }
    let client_dir = ws.root.join(client_name);
    if let Ok(content) = fs::read_to_string(client_dir.join("config.toml")) {
        if let Ok(config) = toml::from_str::<ClientConfig>(&content) {
            if !config.client.sow.template.is_empty() {
                return Some(config.client.sow.template.clone());
            }
        }
    }
    None
}

/// Increment the requirement sequence counter and return the new ID.
pub fn increment_requirement_sequence(project_dir: &Utf8PathBuf) -> Result<String, WorkspaceError> {
    let config_path = project_dir.join("config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut config: ProjectConfig = toml::from_str(&content)?;

    let seq = config
        .project
        .id
        .as_mut()
        .ok_or(WorkspaceError::MissingField(
            "project config missing [project.id] section".to_string(),
        ))?;

    let padding = seq.padding as usize;
    let id = format!(
        "REQ-{:0>padding$}",
        seq.requirement_sequence,
        padding = padding
    );
    seq.requirement_sequence += 1;

    let toml = toml::to_string_pretty(&config)?;
    crate::atomic_write(&config_path, toml.as_bytes())?;

    Ok(id)
}

/// Check if a requirement status transition is valid per the spec state machine.
pub fn is_valid_requirement_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        ("open", "in_progress")
            | ("in_progress", "verified")
            | ("in_progress", "rejected")
            | ("open", "deferred")
            | ("deferred", "open")
    )
}

/// Required frontmatter fields per entity type.
pub const FINDING_REQUIRED_FIELDS: &[&str] = &["id", "status", "severity", "created", "updated"];
pub const REQUIREMENT_REQUIRED_FIELDS: &[&str] = &["id", "status", "created", "updated"];

/// Check frontmatter for missing required fields. Returns list of missing field names.
pub fn check_missing_frontmatter_fields(
    frontmatter: &serde_yaml::Value,
    required: &[&str],
) -> Vec<String> {
    let mut missing = Vec::new();
    for field in required {
        let val = frontmatter.get(*field);
        if val.is_none() || val == Some(&serde_yaml::Value::Null) {
            missing.push(field.to_string());
        }
    }
    missing
}

/// Valid finding statuses per the spec.
pub const VALID_FINDING_STATUSES: &[&str] = &[
    "open",
    "fixed",
    "false_positive",
    "not_applicable",
    "risk_accepted",
];

/// Valid requirement statuses per the spec.
pub const VALID_REQUIREMENT_STATUSES: &[&str] =
    &["open", "in_progress", "verified", "rejected", "deferred"];

/// Valid severity levels per the spec.
pub const VALID_SEVERITIES: &[&str] = &["critical", "high", "medium", "low", "informational"];

/// Check if a value is in a list of valid values.
pub fn is_valid_value(value: &str, valid: &[&str]) -> bool {
    valid.contains(&value)
}

/// Validate a date string is in YYYY-MM-DD format.
pub fn is_valid_date(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn resolve_path_rejects_unsafe_segments() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();

        for path in [
            "../acme",
            "acme/../initial",
            "/tmp/acme",
            "acme//initial",
            "acme/./initial",
        ] {
            assert!(
                resolve_path(&ws, path).is_err(),
                "accepted unsafe path: {path}"
            );
        }
    }

    #[test]
    fn create_and_list_clients() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        create_client(&ws, "acme").unwrap();
        create_client(&ws, "foobar").unwrap();

        let clients = list_entities(&ws.root, 1).unwrap();
        assert_eq!(clients, vec!["acme", "foobar"]);
    }

    #[test]
    fn create_project_under_client() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        create_client(&ws, "acme").unwrap();
        let proj = create_project(&ws, "acme", "web_app").unwrap();
        assert!(proj.join("config.toml").exists());

        let projects = list_entities(&ws.root.join("acme"), 2).unwrap();
        assert_eq!(projects, vec!["web_app"]);
    }

    #[test]
    fn create_engagement_under_project() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        create_client(&ws, "acme").unwrap();
        create_project(&ws, "acme", "web_app").unwrap();
        let eng = create_engagement(&ws, "acme", "web_app", "initial", None, None).unwrap();
        assert!(eng.join("config.toml").exists());
    }

    #[test]
    fn duplicate_client_rejected() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        create_client(&ws, "acme").unwrap();
        assert!(create_client(&ws, "acme").is_err());
    }

    #[test]
    fn project_without_client_rejected() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        assert!(create_project(&ws, "nonexistent", "web_app").is_err());
    }

    #[test]
    fn invalid_name_rejected() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        assert!(create_client(&ws, "Acme").is_err());
        assert!(create_client(&ws, "acme corp").is_err());
        assert!(create_client(&ws, "templates").is_err());
    }

    #[test]
    fn show_config_works() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        create_client(&ws, "acme").unwrap();
        let content = show_config(&ws, "acme").unwrap();
        assert!(content.contains("[client]"));
    }

    #[test]
    fn remove_entity_works() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
        let dir = create_client(&ws, "acme").unwrap();
        assert!(dir.exists());
        remove_entity(&ws, "acme").unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn increment_sequence_works() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();
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
        let tw = TestWorkspace::new();
        // Manually create a client dir with config.toml (using Default impl)
        let dir = tw.root.join("manual_client");
        fs::create_dir_all(&dir).unwrap();
        let mut config = ClientConfig::default();
        config.client.id = Some(ClientIdSection {
            prefix: "MAN".to_string(),
        });
        let toml = toml::to_string_pretty(&config).unwrap();
        fs::write(dir.join("config.toml"), toml).unwrap();

        // list_entities should find it
        let clients = list_entities(&tw.root, 1).unwrap();
        assert!(clients.contains(&"manual_client".to_string()));
    }

    #[test]
    fn update_engagement_status_works() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        update_engagement_status(&ws, "acme/web/initial", "planned").unwrap();
        let (eng_dir, _) = resolve_existing_entity(&ws, "acme/web/initial").unwrap();
        let config: EngagementConfig =
            toml::from_str(&std::fs::read_to_string(eng_dir.join("config.toml")).unwrap()).unwrap();
        assert_eq!(config.engagement.status, "planned");
    }

    #[test]
    fn invalid_status_rejected() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        let result = update_engagement_status(&ws, "acme/web/initial", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn credential_gate_blocks_in_progress() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");

        // Enable gate in workspace config
        let ws = tw.workspace();
        let mut config = ws.config.clone();
        config.workspace.require_credentials_ready = true;
        let toml = toml::to_string_pretty(&config).unwrap();
        std::fs::write(ws.root.join("config.toml"), toml).unwrap();
        let ws = Workspace::load(&ws.root).unwrap();

        // Try to move to in_progress — should fail
        let result = update_engagement_status(&ws, "acme/web/initial", "in_progress");
        assert!(result.is_err());

        // Toggle credentials ready
        let ready = toggle_credentials_ready(&ws, "acme/web/initial").unwrap();
        assert!(ready);

        // Now should succeed
        update_engagement_status(&ws, "acme/web/initial", "in_progress").unwrap();
    }

    #[test]
    fn toggle_credentials_ready_works() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        let val1 = toggle_credentials_ready(&ws, "acme/web/initial").unwrap();
        assert!(val1);
        let val2 = toggle_credentials_ready(&ws, "acme/web/initial").unwrap();
        assert!(!val2);
    }

    #[test]
    fn update_engagement_date_works() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        update_engagement_date(&ws, "acme/web/initial", "start_date", "2026-08-01").unwrap();
        let (eng_dir, _) = resolve_existing_entity(&ws, "acme/web/initial").unwrap();
        let config: EngagementConfig =
            toml::from_str(&std::fs::read_to_string(eng_dir.join("config.toml")).unwrap()).unwrap();
        assert_eq!(config.engagement.start_date, "2026-08-01");
    }

    #[test]
    fn invalid_date_rejected() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_engagement("acme", "web", "initial");
        let ws = tw.workspace();

        let result = update_engagement_date(&ws, "acme/web/initial", "start_date", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn list_client_engagements_returns_all() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        tw.create_project("acme", "api");
        tw.create_engagement("acme", "web", "initial");
        tw.create_engagement("acme", "web", "retest_01");
        tw.create_engagement("acme", "api", "v1_assess");
        let ws = tw.workspace();

        // Set status on one engagement
        update_engagement_status(&ws, "acme/web/initial", "in_progress").unwrap();
        update_engagement_date(&ws, "acme/web/initial", "start_date", "2026-07-01").unwrap();
        update_engagement_date(&ws, "acme/web/initial", "end_date", "2026-07-14").unwrap();

        let summaries = list_client_engagements(&ws, "acme").unwrap();
        assert_eq!(summaries.len(), 3);

        // Sorted by project then name
        assert_eq!(summaries[0].client, "acme");
        assert_eq!(summaries[0].project, "api");
        assert_eq!(summaries[0].name, "v1_assess");
        assert_eq!(summaries[0].status, "draft");

        assert_eq!(summaries[1].client, "acme");
        assert_eq!(summaries[1].project, "web");
        assert_eq!(summaries[1].name, "initial");
        assert_eq!(summaries[1].status, "in_progress");
        assert_eq!(summaries[1].start_date, "2026-07-01");
        assert_eq!(summaries[1].end_date, "2026-07-14");

        assert_eq!(summaries[2].client, "acme");
        assert_eq!(summaries[2].project, "web");
        assert_eq!(summaries[2].name, "retest_01");
        assert_eq!(summaries[2].status, "draft");
    }

    #[test]
    fn list_client_engagements_nonexistent_client() {
        let tw = TestWorkspace::new();
        let ws = tw.workspace();

        let result = list_client_engagements(&ws, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn list_client_engagements_empty_client() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        let ws = tw.workspace();

        let summaries = list_client_engagements(&ws, "acme").unwrap();
        assert!(summaries.is_empty());
    }
}
