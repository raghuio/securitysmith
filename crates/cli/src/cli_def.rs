// cli_def.rs — CLI definitions shared by main and gen-man.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sm")]
#[command(about = "CLI for managing security projects and engagements")]
#[command(version)]
pub struct Cli {
    /// Workspace by name (from global config) or path
    #[arg(short, long, global = true)]
    pub workspace: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new workspace, client, project, engagement, or template
    #[command(visible_alias = "n")]
    New {
        /// Path or name (no args = workspace, absolute path = workspace, single name = client, multi-segment = hierarchy)
        path: Option<String>,
    },

    /// List entities
    #[command(visible_alias = "l")]
    Ls {
        /// Path to list children of
        path: Option<String>,
        /// List findings only
        #[arg(long)]
        findings: bool,
        /// List requirements only
        #[arg(long)]
        requirements: bool,
        /// List notes only
        #[arg(long)]
        notes: bool,
        /// Show scope.md
        #[arg(long)]
        scope: bool,
        /// Filter findings by severity
        #[arg(long)]
        severity: Option<String>,
        /// Filter findings by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Show entity details
    #[command(visible_alias = "s")]
    Show {
        /// Path to entity
        path: String,
    },

    /// Open config in $EDITOR
    #[command(visible_alias = "e")]
    Edit {
        /// Path to entity
        path: String,
    },

    /// Remove an entity (requires --force)
    #[command(visible_alias = "r")]
    Rm {
        /// Path to entity
        path: String,
        /// Confirm removal
        #[arg(long)]
        force: bool,
    },

    /// Show current workspace info
    #[command(visible_alias = "st")]
    Status,

    /// Check workspace health
    #[command(visible_alias = "c")]
    Check {
        /// Remove stale workspace entries from global config
        #[arg(long)]
        fix: bool,
    },

    /// Show or set global configuration
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Show workspace or client statistics
    Stats {
        /// Client name for per-client stats
        client: Option<String>,
        /// Aggregate across all known workspaces
        #[arg(long)]
        all: bool,
    },

    /// Manage findings
    #[command(visible_alias = "f")]
    Finding {
        /// Engagement path or finding ID
        path_or_id: String,
        /// Title for new finding
        #[arg(long)]
        title: Option<String>,
        /// Update finding status
        #[arg(long)]
        status: Option<String>,
        /// Update finding severity
        #[arg(long)]
        severity: Option<String>,
        /// Export format (markdown, html, pdf, json)
        #[arg(long)]
        export: Option<String>,
        /// Output path for export
        #[arg(long)]
        to: Option<String>,
        /// Skip template
        #[arg(long)]
        no_template: bool,
    },

    /// Manage requirements
    Req {
        /// Engagement path or requirement ID
        path_or_id: String,
        /// Title for new requirement
        #[arg(long)]
        title: Option<String>,
        /// Update requirement status
        #[arg(long)]
        status: Option<String>,
        /// Skip template
        #[arg(long)]
        no_template: bool,
    },

    /// Open scope.md in editor
    Scope {
        /// Engagement path
        path: String,
    },

    /// Create a quick note
    Note {
        /// Engagement path
        path: String,
        /// Note message
        message: String,
    },

    /// Build a report
    Report {
        /// Engagement, project, or client path
        path: String,
        /// Output format (markdown, html, pdf, json)
        #[arg(long)]
        format: Option<String>,
        /// Template slug
        #[arg(long)]
        template: Option<String>,
        /// Output file path
        #[arg(long)]
        to: Option<String>,
    },

    /// Build a SOW
    Sow {
        /// Engagement, project, or client path
        path: String,
        /// Output format (markdown, html, pdf, json)
        #[arg(long)]
        format: Option<String>,
        /// Template slug
        #[arg(long)]
        template: Option<String>,
        /// Output file path
        #[arg(long)]
        to: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key (e.g. default_workspace)
        key: String,
        /// Configuration value
        value: String,
    },
}
