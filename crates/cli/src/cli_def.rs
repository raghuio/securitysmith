#![allow(clippy::collapsible_if)]
// cli_def.rs — CLI definitions shared by main and gen-man.

use clap::{Parser, Subcommand, ValueEnum};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SeverityArg {
    #[value(name = "critical")]
    Critical,
    #[value(name = "high")]
    High,
    #[value(name = "medium")]
    Medium,
    #[value(name = "low")]
    Low,
    #[value(name = "informational")]
    Informational,
}

impl SeverityArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Informational => "informational",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FindingStatusArg {
    #[value(name = "open")]
    Open,
    #[value(name = "fixed")]
    Fixed,
    #[value(name = "false_positive")]
    FalsePositive,
    #[value(name = "not_applicable")]
    NotApplicable,
    #[value(name = "risk_accepted")]
    RiskAccepted,
}

impl FindingStatusArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Fixed => "fixed",
            Self::FalsePositive => "false_positive",
            Self::NotApplicable => "not_applicable",
            Self::RiskAccepted => "risk_accepted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RequirementStatusArg {
    #[value(name = "open")]
    Open,
    #[value(name = "in_progress")]
    InProgress,
    #[value(name = "verified")]
    Verified,
    #[value(name = "rejected")]
    Rejected,
    #[value(name = "deferred")]
    Deferred,
}

impl RequirementStatusArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Verified => "verified",
            Self::Rejected => "rejected",
            Self::Deferred => "deferred",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    #[value(name = "markdown")]
    Markdown,
    #[value(name = "html")]
    Html,
    #[value(name = "pdf")]
    Pdf,
    #[value(name = "json")]
    Json,
}

impl FormatArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Pdf => "pdf",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EngagementStatusArg {
    #[value(name = "draft")]
    Draft,
    #[value(name = "planned")]
    Planned,
    #[value(name = "in_progress")]
    InProgress,
    #[value(name = "paused")]
    Paused,
    #[value(name = "completed")]
    Completed,
    #[value(name = "closed")]
    Closed,
}

impl EngagementStatusArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocumentTypeArg {
    #[value(name = "roe")]
    Roe,
    #[value(name = "nda")]
    Nda,
    #[value(name = "proposal")]
    Proposal,
    #[value(name = "custom")]
    Custom,
}

impl DocumentTypeArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Roe => "roe",
            Self::Nda => "nda",
            Self::Proposal => "proposal",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SearchTypeArg {
    #[value(name = "finding")]
    Finding,
    #[value(name = "requirement")]
    Requirement,
    #[value(name = "note")]
    Note,
    #[value(name = "scope")]
    Scope,
    #[value(name = "template")]
    Template,
    #[value(name = "document")]
    Document,
}

impl SearchTypeArg {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Finding => "finding",
            Self::Requirement => "requirement",
            Self::Note => "note",
            Self::Scope => "scope",
            Self::Template => "template",
            Self::Document => "document",
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new workspace, client, project, engagement, or template
    #[command(visible_alias = "n")]
    New {
        /// Path or name (no args = workspace, absolute path = workspace, single name = client, multi-segment = hierarchy)
        path: Option<String>,
        /// Engagement start date (YYYY-MM-DD) — only for engagement creation (depth 3)
        #[arg(long)]
        start: Option<String>,
        /// Engagement end date (YYYY-MM-DD) — only for engagement creation (depth 3)
        #[arg(long)]
        end: Option<String>,
    },

    /// List entities
    #[command(visible_alias = "l", visible_alias = "list")]
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
        /// List document sections
        #[arg(long)]
        sections: bool,
        /// List custom documents
        #[arg(long)]
        documents: bool,
        /// Filter findings by severity
        #[arg(long, value_enum)]
        severity: Option<SeverityArg>,
        /// Filter findings by status
        #[arg(long, value_enum)]
        status: Option<FindingStatusArg>,
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

    /// Remove an entity (prompts for confirmation unless --yes)
    #[command(visible_alias = "r")]
    Rm {
        /// Path to entity
        path: String,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Show workspace info, or list engagements for a client
    #[command(visible_alias = "st")]
    Status {
        /// Client name to show engagement status for
        client: Option<String>,
        /// Show archived engagements only (completed, closed)
        #[arg(long)]
        archived: bool,
        /// Show all engagements (active + archived)
        #[arg(long)]
        all: bool,
    },

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

    /// Manage methodology checklists
    #[command(visible_alias = "cl")]
    Checklist {
        /// Engagement path
        path: String,
        /// Assign a built-in checklist (e.g., owasp-wstg)
        #[arg(long)]
        assign: Option<String>,
        /// List items with status and coverage
        #[arg(long)]
        list: bool,
        /// Checklist item ID to update
        #[arg(long)]
        item: Option<String>,
        /// Update item status
        #[arg(long)]
        status: Option<String>,
        /// Link a finding ID to an item
        #[arg(long)]
        finding: Option<String>,
    },

    /// Search workspace Markdown files
    Search {
        /// Search query
        query: String,
        /// Filter by entity type (finding, requirement, note, scope, template, document)
        #[arg(long, value_enum)]
        r#type: Option<SearchTypeArg>,
        /// Limit to a specific client
        #[arg(long)]
        client: Option<String>,
    },

    /// Manage credentials (encrypted store)
    #[command(visible_alias = "cred")]
    Credential {
        /// Engagement path or credential ID
        path_or_id: String,
        /// Add a credential (requires --label and --type)
        #[arg(long)]
        add: bool,
        /// Credential label
        #[arg(long)]
        label: Option<String>,
        /// Credential type (url, username_password, api_key, bearer_token, vpn_config, ssh_key, custom)
        #[arg(long)]
        cred_type: Option<String>,
        /// List credentials for an engagement
        #[arg(long)]
        list: bool,
        /// Show full credential (including value)
        #[arg(long)]
        show: bool,
        /// Update credential status (not_verified, working, not_working, expired)
        #[arg(long)]
        status: Option<String>,
        /// Remove credential
        #[arg(long)]
        rm: bool,
    },

    /// Manage evidence files
    #[command(visible_alias = "ev")]
    Evidence {
        /// Engagement path
        path: String,
        /// Add a file to evidence
        #[arg(long)]
        add: Option<String>,
        /// List evidence files
        #[arg(long)]
        list: bool,
        /// Show/open an evidence file
        #[arg(long)]
        show: Option<String>,
    },

    /// Manage engagement status and fields
    #[command(visible_alias = "eng")]
    Engagement {
        /// Engagement path (client/project/engagement)
        path: String,
        /// Update engagement status (planned, in_progress, paused, completed, closed)
        #[arg(long, value_enum)]
        status: Option<EngagementStatusArg>,
        /// Update start date (YYYY-MM-DD)
        #[arg(long)]
        start_date: Option<String>,
        /// Update end date (YYYY-MM-DD)
        #[arg(long)]
        end_date: Option<String>,
        /// Toggle credential readiness
        #[arg(long)]
        credentials_ready: bool,
        /// Create a retest engagement (use with --from)
        #[arg(long)]
        retest: bool,
        /// Original engagement to copy findings from (for --retest)
        #[arg(long)]
        from: Option<String>,
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
        #[arg(long, value_enum)]
        status: Option<FindingStatusArg>,
        /// Update finding severity
        #[arg(long, value_enum)]
        severity: Option<SeverityArg>,
        /// Export format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        export: Option<FormatArg>,
        /// Output path for export
        #[arg(long)]
        to: Option<String>,
        /// Skip template
        #[arg(long)]
        no_template: bool,
        /// Import findings from a file
        #[arg(long)]
        import: Option<String>,
        /// Import format (nessus, csv)
        #[arg(long)]
        import_format: Option<String>,
        /// CSV title column index (0-based)
        #[arg(long)]
        title_column: Option<usize>,
        /// CSV severity column index (0-based)
        #[arg(long)]
        severity_column: Option<usize>,
        /// Update retest result (not_tested, pass, fail, partial)
        #[arg(long)]
        retest_result: Option<String>,
        /// Update client response (acknowledged, in_progress, fixed, accepted_risk, disputed, deferred, no_response)
        #[arg(long)]
        client_response: Option<String>,
        /// Set fix deadline (YYYY-MM-DD or 'auto' to calculate from severity)
        #[arg(long)]
        fix_deadline: Option<String>,
    },

    /// Manage requirements
    #[command(visible_alias = "requirement")]
    Req {
        /// Engagement path or requirement ID
        path_or_id: String,
        /// Title for new requirement
        #[arg(long)]
        title: Option<String>,
        /// Update requirement status
        #[arg(long, value_enum)]
        status: Option<RequirementStatusArg>,
        /// Export format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        export: Option<FormatArg>,
        /// Output path for export
        #[arg(long)]
        to: Option<String>,
        /// Skip template
        #[arg(long)]
        no_template: bool,
    },

    /// Open scope.md in editor or export
    Scope {
        /// Engagement path
        path: String,
        /// Export format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        export: Option<FormatArg>,
        /// Output path for export
        #[arg(long)]
        to: Option<String>,
    },

    /// Create a quick note or export notes
    Note {
        /// Engagement path
        path: String,
        /// Note message (omit when using --export)
        message: Option<String>,
        /// Export format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        export: Option<FormatArg>,
        /// Output path for export
        #[arg(long)]
        to: Option<String>,
    },

    /// Build a report
    Report {
        /// Engagement, project, or client path
        path: Option<String>,
        /// Output format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        format: Option<FormatArg>,
        /// Template slug
        #[arg(long)]
        template: Option<String>,
        /// Output file path
        #[arg(long)]
        to: Option<String>,
        /// Override which sections to include (comma-separated, e.g. web/methodology,pricing)
        #[arg(long)]
        sections: Option<String>,
        /// Exclude specific sections (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
        /// Aggregate across all known workspaces
        #[arg(long)]
        all: bool,
    },

    /// Build a SOW
    Sow {
        /// Engagement, project, or client path
        path: Option<String>,
        /// Output format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        format: Option<FormatArg>,
        /// Template slug
        #[arg(long)]
        template: Option<String>,
        /// Output file path
        #[arg(long)]
        to: Option<String>,
        /// Override which sections to include (comma-separated)
        #[arg(long)]
        sections: Option<String>,
        /// Exclude specific sections (comma-separated)
        #[arg(long)]
        exclude: Option<String>,
        /// Aggregate across all known workspaces
        #[arg(long)]
        all: bool,
    },

    /// Manage custom documents (RoE, NDA, proposal, custom)
    #[command(visible_alias = "doc")]
    Document {
        /// Client path, engagement path, or document ID
        path_or_id: String,
        /// Title for new document
        #[arg(long)]
        title: Option<String>,
        /// Document type (roe, nda, proposal, custom)
        #[arg(long, value_enum)]
        doc_type: Option<DocumentTypeArg>,
        /// Finalize document (set read-only)
        #[arg(long)]
        finalize: bool,
        /// Unlock document (revert to draft)
        #[arg(long)]
        unlock: bool,
        /// Export format (markdown, html, pdf, json)
        #[arg(long, value_enum)]
        export: Option<FormatArg>,
        /// Output path for export
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
