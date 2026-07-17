#![allow(clippy::collapsible_if)]
use clap::Parser;
use ss_workspace::{Workspace, global::GlobalConfig};

mod cli_def;
mod color;
mod error;
mod hardening;

use cli_def::{Cli, Commands, ConfigAction};
use error::{exit_code, fail, workspace_error};

fn main() {
    let cli = Cli::parse();

    // Determine if this command needs $EDITOR (for pledge/seccomp allowlist).
    let needs_editor = needs_editor(&cli.command);

    // Try to find the workspace root before applying hardening.
    // This enables unveil() on OpenBSD to restrict filesystem access.
    // Best-effort: if the workspace can't be found yet, hardening proceeds without unveil.
    let ws_root = try_workspace_root(cli.workspace.as_deref());

    // Apply platform-specific hardening (pledge/unveil on OpenBSD, seccomp on Linux).
    // Best-effort: if hardening can't be applied, the tool continues without it.
    hardening::apply(ws_root.as_deref().map(|p| p.as_std_path()), needs_editor);

    let result = run(cli);
    if let Err((msg, code)) = result {
        fail(&msg, code);
    }
}

/// Best-effort workspace root resolution for hardening (unveil on OpenBSD).
/// Tries: -w flag > current directory > default workspace.
/// Returns None if no workspace can be found.
fn try_workspace_root(ws_flag: Option<&str>) -> Option<camino::Utf8PathBuf> {
    // 1. Try explicit -w flag
    if let Some(spec) = ws_flag {
        if let Ok(global) = ss_workspace::global::GlobalConfig::load()
            && let Some(ws) = global.find_workspace(spec)
        {
            return Some(ws.path.clone());
        }
        let path = ss_workspace::expand_tilde(spec);
        if path.join(ss_workspace::CONFIG_FILE).exists() {
            return Some(path);
        }
    }

    // 2. Try walking up from current directory
    if let Ok(cwd) = ss_workspace::current_dir() {
        let mut current = cwd;
        loop {
            if current.join(ss_workspace::CONFIG_FILE).exists() {
                return Some(current);
            }
            match current.parent() {
                Some(p) => current = p.to_path_buf(),
                None => break,
            }
        }
    }

    // 3. Try default workspace
    if let Ok(global) = ss_workspace::global::GlobalConfig::load()
        && let Some(default) = global.default_workspace()
    {
        return Some(default);
    }

    None
}

/// Determine if the command needs to exec $EDITOR.
fn needs_editor(cmd: &Commands) -> bool {
    match cmd {
        Commands::Edit { .. } => true,
        Commands::Scope { export: None, .. } => true,
        Commands::Finding { title: Some(_), .. } => true,
        Commands::Req { title: Some(_), .. } => true,
        Commands::Document { title: Some(_), .. } => true,
        Commands::New { path: Some(p), .. } => p.starts_with("templates/"),
        Commands::Status { .. } => false,
        _ => false,
    }
}

fn run(cli: Cli) -> Result<(), (String, i32)> {
    match cli.command {
        Commands::New { path, start, end } => cmd_new(
            path,
            cli.workspace.as_deref(),
            start.as_deref(),
            end.as_deref(),
        ),
        Commands::Status {
            client,
            archived,
            all,
        } => cmd_status(cli.workspace.as_deref(), client.as_deref(), archived, all),
        Commands::Config { action } => cmd_config(action),
        Commands::Check { fix } => cmd_check(fix, cli.workspace.as_deref()),
        Commands::Ls {
            path,
            findings,
            requirements,
            notes,
            scope,
            sections,
            documents,
            severity,
            status,
        } => cmd_ls(
            cli.workspace.as_deref(),
            path.as_deref(),
            findings,
            requirements,
            notes,
            scope,
            sections,
            documents,
            severity.map(|s| s.as_str()),
            status.map(|s| s.as_str()),
        ),
        Commands::Show { path } => cmd_show(cli.workspace.as_deref(), &path),
        Commands::Edit { path } => cmd_edit(cli.workspace.as_deref(), &path),
        Commands::Rm { path, yes } => cmd_rm(cli.workspace.as_deref(), &path, yes),
        Commands::Stats { client, all } => {
            cmd_stats(cli.workspace.as_deref(), client.as_deref(), all)
        }
        Commands::Finding {
            path_or_id,
            title,
            status,
            severity,
            export,
            to,
            no_template,
            import,
            import_format,
            title_column,
            severity_column,
            retest_result,
            client_response,
            fix_deadline,
        } => cmd_finding(
            cli.workspace.as_deref(),
            &path_or_id,
            title.as_deref(),
            status.map(|s| s.as_str()),
            severity.map(|s| s.as_str()),
            export.map(|f| f.as_str()),
            to.as_deref(),
            no_template,
            import.as_deref(),
            import_format.as_deref(),
            title_column,
            severity_column,
            retest_result.as_deref(),
            client_response.as_deref(),
            fix_deadline.as_deref(),
        ),
        Commands::Req {
            path_or_id,
            title,
            status,
            export,
            to,
            no_template,
        } => cmd_req(
            cli.workspace.as_deref(),
            &path_or_id,
            title.as_deref(),
            status.map(|s| s.as_str()),
            export.map(|f| f.as_str()),
            to.as_deref(),
            no_template,
        ),
        Commands::Scope { path, export, to } => cmd_scope(
            cli.workspace.as_deref(),
            &path,
            export.map(|f| f.as_str()),
            to.as_deref(),
        ),
        Commands::Note {
            path,
            message,
            export,
            to,
        } => cmd_note(
            cli.workspace.as_deref(),
            &path,
            message.as_deref(),
            export.map(|f| f.as_str()),
            to.as_deref(),
        ),
        Commands::Report {
            path,
            format,
            template,
            to,
            sections,
            exclude,
            all,
        } => cmd_report(
            cli.workspace.as_deref(),
            path.as_deref(),
            format.map(|f| f.as_str()),
            template.as_deref(),
            to.as_deref(),
            sections.as_deref(),
            exclude.as_deref(),
            all,
        ),
        Commands::Sow {
            path,
            format,
            template,
            to,
            sections,
            exclude,
            all,
        } => cmd_sow(
            cli.workspace.as_deref(),
            path.as_deref(),
            format.map(|f| f.as_str()),
            template.as_deref(),
            to.as_deref(),
            sections.as_deref(),
            exclude.as_deref(),
            all,
        ),
        Commands::Engagement {
            path,
            status,
            start_date,
            end_date,
            credentials_ready,
            retest,
            from,
        } => cmd_engagement(
            cli.workspace.as_deref(),
            &path,
            status.map(|s| s.as_str()),
            start_date.as_deref(),
            end_date.as_deref(),
            credentials_ready,
            retest,
            from.as_deref(),
        ),
        Commands::Checklist {
            path,
            assign,
            list,
            item,
            status,
            finding,
        } => cmd_checklist(
            cli.workspace.as_deref(),
            &path,
            assign.as_deref(),
            list,
            item.as_deref(),
            status.as_deref(),
            finding.as_deref(),
        ),
        Commands::Search {
            query,
            r#type,
            client,
        } => cmd_search(
            cli.workspace.as_deref(),
            &query,
            r#type.map(|t| t.as_str()),
            client.as_deref(),
        ),
        Commands::Credential {
            path_or_id,
            add,
            label,
            cred_type,
            list,
            show,
            status,
            rm,
        } => cmd_credential(
            cli.workspace.as_deref(),
            &path_or_id,
            add,
            label.as_deref(),
            cred_type.as_deref(),
            list,
            show,
            status.as_deref(),
            rm,
        ),
        Commands::Evidence {
            path,
            add,
            list,
            show,
        } => cmd_evidence(
            cli.workspace.as_deref(),
            &path,
            add.as_deref(),
            list,
            show.as_deref(),
        ),
        Commands::Document {
            path_or_id,
            title,
            doc_type,
            finalize,
            unlock,
            export,
            to,
        } => cmd_document(
            cli.workspace.as_deref(),
            &path_or_id,
            title.as_deref(),
            doc_type.map(|t| t.as_str()),
            finalize,
            unlock,
            export.map(|f| f.as_str()),
            to.as_deref(),
        ),
    }
}

// ── Workspace commands ──────────────────────────────

fn cmd_new(
    path: Option<String>,
    ws_flag: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(), (String, i32)> {
    match path {
        None => {
            let cwd = ss_workspace::current_dir().map_err(|e| workspace_error(&e))?;
            create_workspace(&cwd)
        }
        Some(p) if ss_workspace::is_absolute_path(&p) => {
            let target = ss_workspace::expand_tilde(&p);
            create_workspace(&target)
        }
        Some(p) => {
            let segments: Vec<&str> = p.split('/').collect();

            // Template creation
            if segments[0] == "templates" {
                if segments.len() == 1 {
                    return Err((
                        "Name `templates` is reserved.".to_string(),
                        exit_code::RESERVED_NAME,
                    ));
                }
                if segments.len() != 2 {
                    return Err((
                        "Usage: sm new templates/<type>. Types: finding, report, sow, requirement, workspace, client, project, engagement"
                            .to_string(),
                        exit_code::INVALID_NAME_FORMAT,
                    ));
                }
                let ws = resolve_workspace(ws_flag)?;
                // Strip .md or .toml extension if user included it
                let template_name = segments[1]
                    .strip_suffix(".md")
                    .or_else(|| segments[1].strip_suffix(".toml"))
                    .unwrap_or(segments[1]);
                return create_template(&ws, template_name);
            }

            // Entity creation
            let ws = resolve_workspace(ws_flag)?;
            match segments.len() {
                1 => {
                    let dir = ss_workspace::entities::create_client(&ws, segments[0])
                        .map_err(|e| workspace_error(&e))?;
                    println!("Created client: {}", dir);
                }
                2 => {
                    // Auto-create missing parent client
                    if ss_workspace::entities::resolve_existing_entity(&ws, segments[0]).is_err() {
                        let client_dir = ss_workspace::entities::create_client(&ws, segments[0])
                            .map_err(|e| workspace_error(&e))?;
                        println!("Created client: {}", client_dir);
                    }
                    let dir = ss_workspace::entities::create_project(&ws, segments[0], segments[1])
                        .map_err(|e| workspace_error(&e))?;
                    println!("Created project: {}", dir);
                }
                3 => {
                    // Auto-create missing parent client
                    if ss_workspace::entities::resolve_existing_entity(&ws, segments[0]).is_err() {
                        let client_dir = ss_workspace::entities::create_client(&ws, segments[0])
                            .map_err(|e| workspace_error(&e))?;
                        println!("Created client: {}", client_dir);
                    }
                    // Auto-create missing parent project
                    let project_path = format!("{}/{}", segments[0], segments[1]);
                    if ss_workspace::entities::resolve_existing_entity(&ws, &project_path).is_err()
                    {
                        let project_dir =
                            ss_workspace::entities::create_project(&ws, segments[0], segments[1])
                                .map_err(|e| workspace_error(&e))?;
                        println!("Created project: {}", project_dir);
                    }
                    let dir = ss_workspace::entities::create_engagement(
                        &ws,
                        segments[0],
                        segments[1],
                        segments[2],
                        start,
                        end,
                    )
                    .map_err(|e| workspace_error(&e))?;
                    println!("Created engagement: {}", dir);
                }
                _ => {
                    return Err((
                        "Path too deep. Max depth is 3 (client/project/engagement).".to_string(),
                        exit_code::INVALID_NAME_FORMAT,
                    ));
                }
            }
            Ok(())
        }
    }
}

fn create_workspace(path: &camino::Utf8PathBuf) -> Result<(), (String, i32)> {
    let ws = Workspace::init(path).map_err(|e| workspace_error(&e))?;

    let mut global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
    let name = ws.config.workspace.name.clone();
    global.register_workspace(&name, &ws.root);
    global.save().map_err(|e| workspace_error(&e))?;

    println!("Created workspace at {}", ws.root);
    println!("Name: {}", ws.config.workspace.name);
    println!("Suggest: cd {} && git init", ws.root);
    Ok(())
}

fn cmd_status(
    ws_flag: Option<&str>,
    client: Option<&str>,
    archived: bool,
    all: bool,
) -> Result<(), (String, i32)> {
    // If both --archived and --all are set, --all wins.
    let filter = if all {
        StatusFilter::All
    } else if archived {
        StatusFilter::Archived
    } else {
        StatusFilter::Active
    };

    if let Some(client_name) = client {
        // Single client in current (or -w) workspace
        let ws = resolve_workspace(ws_flag)?;
        let summaries = ss_workspace::entities::list_client_engagements(&ws, client_name)
            .map_err(|e| workspace_error(&e))?;

        let filtered: Vec<_> = summaries
            .into_iter()
            .filter(|s| filter.matches(&s.status))
            .collect();

        if filtered.is_empty() {
            println!(
                "No {} engagements found for {}.",
                filter.label(),
                client_name
            );
            return Ok(());
        }

        println!("Client: {}\n", client_name);
        print_engagement_lines(&filtered, false);
        return Ok(());
    }

    // No client argument — show across workspaces
    if ws_flag.is_some() {
        // Single workspace via -w flag
        let ws = resolve_workspace(ws_flag)?;
        let clients =
            ss_workspace::entities::list_entities(&ws.root, 1).map_err(|e| workspace_error(&e))?;

        let mut any = false;
        for client_name in &clients {
            let summaries = ss_workspace::entities::list_client_engagements(&ws, client_name)
                .map_err(|e| workspace_error(&e))?;
            let filtered: Vec<_> = summaries
                .into_iter()
                .filter(|s| filter.matches(&s.status))
                .collect();
            if filtered.is_empty() {
                continue;
            }
            if !any {
                println!("Workspace: {} ({})\n", ws.config.workspace.name, ws.root);
                any = true;
            }
            print_engagement_lines(&filtered, true);
        }
        if !any {
            println!("No {} engagements found.", filter.label());
        }
        return Ok(());
    }

    // All workspaces from global config
    let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
    let mut any = false;

    for ws_entry in &global.workspaces {
        if !ws_entry.path.as_std_path().exists() {
            continue;
        }
        let ws = match ss_workspace::Workspace::load(&ws_entry.path) {
            Ok(ws) => ws,
            Err(_) => continue,
        };

        let clients = ss_workspace::entities::list_entities(&ws.root, 1).unwrap_or_default();

        let mut ws_has_output = false;
        for client_name in &clients {
            let summaries = ss_workspace::entities::list_client_engagements(&ws, client_name)
                .unwrap_or_default();
            let filtered: Vec<_> = summaries
                .into_iter()
                .filter(|s| filter.matches(&s.status))
                .collect();
            if filtered.is_empty() {
                continue;
            }
            if !ws_has_output {
                if any {
                    println!();
                }
                println!("Workspace: {} ({})\n", ws.config.workspace.name, ws.root);
                ws_has_output = true;
                any = true;
            }
            print_engagement_lines(&filtered, true);
        }
    }

    if !any {
        println!("No {} engagements found.", filter.label());
    }
    Ok(())
}

/// Filter mode for engagement status display.
#[derive(Clone, Copy)]
enum StatusFilter {
    Active,
    Archived,
    All,
}

impl StatusFilter {
    fn matches(&self, status: &str) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Archived => status == "completed" || status == "closed",
            StatusFilter::Active => status != "completed" && status != "closed",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            StatusFilter::All => "",
            StatusFilter::Archived => "archived",
            StatusFilter::Active => "active",
        }
    }
}

/// Format a date range string from start and end dates.
fn format_date_range(start: &str, end: &str) -> String {
    if start.is_empty() && end.is_empty() {
        "\u{2014}".to_string() // em dash
    } else if end.is_empty() {
        start.to_string()
    } else if start.is_empty() {
        format!("\u{2192} {}", end) // right arrow
    } else {
        format!("{} \u{2192} {}", start, end)
    }
}

/// Print engagement summaries as one line each.
/// When `show_client` is true, path is `client/project/engagement`.
/// When false, path is `project/engagement` (for single-client view).
fn print_engagement_lines(
    summaries: &[ss_workspace::entities::EngagementSummary],
    show_client: bool,
) {
    for s in summaries {
        let path = if show_client {
            format!("{}/{}/{}", s.client, s.project, s.name)
        } else {
            format!("{}/{}", s.project, s.name)
        };
        let dates = format_date_range(&s.start_date, &s.end_date);
        println!("{:<24} {:<14} {}", path, s.status, dates);
    }
}

fn cmd_config(action: Option<ConfigAction>) -> Result<(), (String, i32)> {
    let mut global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;

    match action {
        None | Some(ConfigAction::Show) => {
            println!(
                "config file: {}",
                ss_workspace::global::config_path().map_err(|e| workspace_error(&e))?
            );
            match global.default_workspace() {
                Some(path) => println!("default_workspace: {}", path),
                None => println!("default_workspace: not set"),
            }
            println!("known workspaces:");
            for ws in &global.workspaces {
                println!("  {} = {}", ws.name, ws.path);
            }
        }
        Some(ConfigAction::Set { key, value }) => match key.as_str() {
            "default_workspace" => {
                let expanded = ss_workspace::expand_tilde(&value);
                global.set_default_workspace(&expanded);
                global.save().map_err(|e| workspace_error(&e))?;
                match global.default_workspace() {
                    Some(p) => println!("Set default_workspace to {}", p),
                    None => println!("Set default_workspace"),
                }
            }
            _ => {
                return Err((
                    format!("Unknown config key: {}", key),
                    exit_code::NOT_A_WORKSPACE,
                ));
            }
        },
    }
    Ok(())
}

fn cmd_check(fix: bool, ws_flag: Option<&str>) -> Result<(), (String, i32)> {
    let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;

    // Check stale workspace entries
    let stale = global.verify_workspaces();
    if stale.is_empty() {
        println!("All workspace entries are valid.");
    } else {
        for ws in &stale {
            println!("Stale: {} at {} (path no longer exists)", ws.name, ws.path);
        }
        if fix {
            let mut global = global;
            let removed = global.remove_stale();
            global.save().map_err(|e| workspace_error(&e))?;
            println!("Removed {} stale entr(ies).", removed);
        }
    }

    // Check current workspace health
    if let Ok(ws) = Workspace::resolve(ws_flag) {
        let issues = ss_workspace::check::check_workspace(&ws);
        if issues.is_empty() {
            println!("Workspace '{}' is healthy.", ws.config.workspace.name);
        } else {
            for issue in &issues {
                let prefix = if issue.severity == ss_workspace::check::IssueSeverity::Error {
                    color::error_prefix("ERROR")
                } else {
                    color::warn_prefix("WARN")
                };
                if let Some(ref path) = issue.path {
                    println!("{}: {} ({})", prefix, issue.message, path);
                } else {
                    println!("{}: {}", prefix, issue.message);
                }
            }
        }
    }

    Ok(())
}

// ── Hierarchy commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_ls(
    ws_flag: Option<&str>,
    path: Option<&str>,
    findings_flag: bool,
    requirements_flag: bool,
    notes_flag: bool,
    scope_flag: bool,
    sections_flag: bool,
    documents_flag: bool,
    severity_filter: Option<&str>,
    status_filter: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    match path {
        None => {
            // List clients
            let clients = ss_workspace::entities::list_entities(&ws.root, 1)
                .map_err(|e| workspace_error(&e))?;
            if clients.is_empty() {
                println!("No clients found.");
            } else {
                for name in clients {
                    println!("{}", name);
                }
            }
        }
        Some("templates") => {
            // List templates
            let entries =
                ss_workspace::templates::list_templates(&ws).map_err(|e| workspace_error(&e))?;
            for entry in entries {
                let src = if entry.source == ss_workspace::templates::TemplateSource::Workspace {
                    "workspace"
                } else {
                    "built-in"
                };
                println!("{}\t{}", entry.name, src);
            }
        }
        Some(p) => {
            ss_workspace::entities::resolve_existing_entity(&ws, p)
                .map_err(|e| workspace_error(&e))?;
            let segments: Vec<&str> = p.split('/').collect();
            match segments.len() {
                1 => {
                    // List projects under client
                    let client_dir = ws.root.join(segments[0]);
                    let projects = ss_workspace::entities::list_entities(&client_dir, 2)
                        .map_err(|e| workspace_error(&e))?;
                    if projects.is_empty() {
                        println!("No projects found under {}.", segments[0]);
                    } else {
                        for name in projects {
                            println!("{}", name);
                        }
                    }
                }
                2 => {
                    // List engagements under project
                    let project_dir = ws.root.join(segments[0]).join(segments[1]);
                    let engagements = ss_workspace::entities::list_entities(&project_dir, 3)
                        .map_err(|e| workspace_error(&e))?;
                    if engagements.is_empty() {
                        println!(
                            "No engagements found under {}/{}.",
                            segments[0], segments[1]
                        );
                    } else {
                        for name in engagements {
                            println!("{}", name);
                        }
                    }
                }
                3 => {
                    // List engagement content
                    let eng_dir = ws
                        .root
                        .join(segments[0])
                        .join(segments[1])
                        .join(segments[2]);
                    if !eng_dir.join("config.toml").exists() {
                        return Err((
                            format!("No engagement matching `{}` found.", p),
                            exit_code::ENTITY_NOT_FOUND,
                        ));
                    }

                    if scope_flag {
                        match ss_workspace::scope::show_scope_content(&ws, p) {
                            Ok(Some(content)) => println!("{}", content),
                            Ok(None) => println!("No scope file found."),
                            Err(e) => return Err(workspace_error(&e)),
                        }
                        return Ok(());
                    }

                    if findings_flag {
                        let entries = ss_workspace::findings::list_findings(
                            &eng_dir,
                            severity_filter,
                            status_filter,
                        )
                        .map_err(|e| workspace_error(&e))?;
                        for entry in entries {
                            println!(
                                "{}\t{}\t{}",
                                entry.id,
                                color::severity(&entry.severity),
                                color::status(&entry.status)
                            );
                        }
                        return Ok(());
                    }

                    if requirements_flag {
                        let entries = ss_workspace::requirements::list_requirements(&eng_dir)
                            .map_err(|e| workspace_error(&e))?;
                        for entry in entries {
                            println!("{}\t{}", entry.id, entry.status);
                        }
                        return Ok(());
                    }

                    if notes_flag {
                        let entries = ss_workspace::notes::list_notes(&eng_dir)
                            .map_err(|e| workspace_error(&e))?;
                        for entry in entries {
                            println!("{}\t{}", entry.id, entry.filename);
                        }
                        return Ok(());
                    }

                    if sections_flag {
                        let eng_type = read_engagement_type(&ws, p);
                        let entries =
                            ss_workspace::sections::list_sections(&ws.root, &eng_dir, &eng_type)
                                .map_err(|e| workspace_error(&e))?;
                        if entries.is_empty() {
                            println!("No document sections found.");
                        } else {
                            for (name, source) in entries {
                                println!("{}\t{}", name, source);
                            }
                        }
                        return Ok(());
                    }

                    if documents_flag {
                        let docs = ss_workspace::documents::list_documents(&eng_dir)
                            .map_err(|e| workspace_error(&e))?;
                        if docs.is_empty() {
                            println!("No custom documents found.");
                        } else {
                            for doc in docs {
                                println!(
                                    "{}\t{}\t{}",
                                    doc.frontmatter.id,
                                    doc.frontmatter.doc_type,
                                    doc.frontmatter.status
                                );
                            }
                        }
                        return Ok(());
                    }

                    // Default: list everything

                    // Findings
                    let f = ss_workspace::findings::list_findings(&eng_dir, None, None)
                        .map_err(|e| workspace_error(&e))?;
                    if !f.is_empty() {
                        println!("Findings:");
                        for entry in &f {
                            println!(
                                "  {}\t{}\t{}",
                                entry.id,
                                color::severity(&entry.severity),
                                color::status(&entry.status)
                            );
                        }
                    }

                    // Requirements
                    let r = ss_workspace::requirements::list_requirements(&eng_dir)
                        .map_err(|e| workspace_error(&e))?;
                    if !r.is_empty() {
                        println!("Requirements:");
                        for entry in &r {
                            println!("  {}\t{}", entry.id, entry.status);
                        }
                    }

                    // Notes
                    let n = ss_workspace::notes::list_notes(&eng_dir)
                        .map_err(|e| workspace_error(&e))?;
                    if !n.is_empty() {
                        println!("Notes:");
                        for entry in &n {
                            println!("  {}\t{}", entry.id, entry.filename);
                        }
                    }

                    // Scope
                    if eng_dir.join("scope.md").exists() {
                        println!("Scope: scope.md exists");
                    }

                    if f.is_empty()
                        && r.is_empty()
                        && n.is_empty()
                        && !eng_dir.join("scope.md").exists()
                    {
                        println!("No content found in this engagement.");
                    }
                }
                _ => {
                    return Err((format!("Path too deep: {}", p), exit_code::ENTITY_NOT_FOUND));
                }
            }
        }
    }
    Ok(())
}

fn cmd_show(ws_flag: Option<&str>, path: &str) -> Result<(), (String, i32)> {
    // Handle templates
    if path.starts_with("templates/") {
        let ws = resolve_workspace(ws_flag)?;
        let name = path
            .strip_prefix("templates/")
            .expect("checked by starts_with above");
        let content =
            ss_workspace::templates::show_template(&ws, name).map_err(|e| workspace_error(&e))?;
        println!("{}", content);
        return Ok(());
    }

    // Handle finding/requirement IDs
    if !path.contains('/') {
        // Could be a finding ID or requirement ID
        let ws = resolve_workspace(ws_flag)?;
        if let Ok(content) = ss_workspace::findings::show_finding(&ws, path) {
            println!("{}", content);
            return Ok(());
        }
        if let Ok(content) = ss_workspace::requirements::show_requirement(&ws, path) {
            println!("{}", content);
            return Ok(());
        }
        let content =
            ss_workspace::entities::show_config(&ws, path).map_err(|e| workspace_error(&e))?;
        println!("{}", content);
        return Ok(());
    }

    // Hierarchy entity — show config.toml
    let ws = resolve_workspace(ws_flag)?;
    let content =
        ss_workspace::entities::show_config(&ws, path).map_err(|e| workspace_error(&e))?;
    println!("{}", content);
    Ok(())
}

fn cmd_edit(ws_flag: Option<&str>, path: &str) -> Result<(), (String, i32)> {
    // Handle templates
    if path.starts_with("templates/") {
        let ws = resolve_workspace(ws_flag)?;
        let name = path
            .strip_prefix("templates/")
            .expect("checked by starts_with above");
        ss_workspace::templates::edit_template(&ws, name).map_err(|e| workspace_error(&e))?;
        return Ok(());
    }

    // Hierarchy entity — edit config.toml
    let ws = resolve_workspace(ws_flag)?;
    ss_workspace::entities::edit_config(&ws, path).map_err(|e| workspace_error(&e))?;
    Ok(())
}

fn cmd_rm(ws_flag: Option<&str>, path: &str, yes: bool) -> Result<(), (String, i32)> {
    // Confirmation prompt unless --yes is set
    if !yes {
        print_confirmation(path);
        if !read_yes_no() {
            return Err((
                "Removal cancelled.".to_string(),
                exit_code::REMOVAL_DECLINED,
            ));
        }
    }

    // Handle templates
    if path.starts_with("templates/") {
        let ws = resolve_workspace(ws_flag)?;
        let name = path
            .strip_prefix("templates/")
            .expect("checked by starts_with above");
        let method =
            ss_workspace::templates::remove_template(&ws, name).map_err(|e| workspace_error(&e))?;
        print_removal("template", name, method);
        return Ok(());
    }

    // Handle finding/requirement IDs (no slashes)
    if !path.contains('/') {
        let ws = resolve_workspace(ws_flag)?;
        // Try finding first
        if ss_workspace::findings::find_finding_file(&ws, path).is_ok() {
            let method = ss_workspace::findings::remove_finding(&ws, path)
                .map_err(|e| workspace_error(&e))?;
            print_removal("finding", path, method);
            return Ok(());
        }
        // Try requirement
        if ss_workspace::requirements::find_requirement_file(&ws, path).is_ok() {
            let method = ss_workspace::requirements::remove_requirement(&ws, path)
                .map_err(|e| workspace_error(&e))?;
            print_removal("requirement", path, method);
            return Ok(());
        }
    }

    // Handle notes: engagement/notes/note_slug
    if path.contains("/notes/") {
        let parts: Vec<&str> = path.split("/notes/").collect();
        if parts.len() == 2 {
            let ws = resolve_workspace(ws_flag)?;
            let method = ss_workspace::notes::remove_note(&ws, parts[0], parts[1])
                .map_err(|e| workspace_error(&e))?;
            print_removal("note", parts[1], method);
            return Ok(());
        }
    }

    // Hierarchy entity
    let ws = resolve_workspace(ws_flag)?;
    let method =
        ss_workspace::entities::remove_entity(&ws, path).map_err(|e| workspace_error(&e))?;
    print_removal("", path, method);
    Ok(())
}

// ── Finding commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_finding(
    ws_flag: Option<&str>,
    path_or_id: &str,
    title: Option<&str>,
    status: Option<&str>,
    severity: Option<&str>,
    export: Option<&str>,
    to: Option<&str>,
    no_template: bool,
    import: Option<&str>,
    import_format: Option<&str>,
    title_column: Option<usize>,
    severity_column: Option<usize>,
    retest_result: Option<&str>,
    client_response: Option<&str>,
    fix_deadline: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Retest/remediation updates
    if let Some(rr) = retest_result {
        ss_workspace::findings::update_remediation_field(&ws, path_or_id, "retest_result", rr)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated {} retest result: {}", path_or_id, rr);
        return Ok(());
    }
    if let Some(cr) = client_response {
        ss_workspace::findings::update_remediation_field(&ws, path_or_id, "client_response", cr)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated {} client response: {}", path_or_id, cr);
        return Ok(());
    }
    if let Some(fd) = fix_deadline {
        let deadline = if fd == "auto" {
            // Get severity from finding
            let content = ss_workspace::findings::show_finding(&ws, path_or_id)
                .map_err(|e| workspace_error(&e))?;
            let parsed = ss_frontmatter::parse(&content).unwrap_or(ss_frontmatter::Parsed {
                frontmatter: serde_yaml::Value::Null,
                body: content.clone(),
            });
            let sev = parsed
                .frontmatter
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");
            ss_workspace::findings::calculate_fix_deadline(sev)
        } else {
            fd.to_string()
        };
        ss_workspace::findings::update_remediation_field(
            &ws,
            path_or_id,
            "fix_deadline",
            &deadline,
        )
        .map_err(|e| workspace_error(&e))?;
        println!("Updated {} fix deadline: {}", path_or_id, deadline);
        return Ok(());
    }

    // Import findings
    if let Some(file) = import {
        let format = import_format.ok_or((
            "--import-format is required (nessus, csv).".to_string(),
            exit_code::INVALID_STATUS_SEVERITY,
        ))?;
        let content = std::fs::read_to_string(file).map_err(|e| {
            (
                format!("Cannot read import file: {e}"),
                exit_code::ENTITY_NOT_FOUND,
            )
        })?;
        let parsed = match format {
            "nessus" => ss_workspace::import::parse_nessus(&content)
                .map_err(|e| (e, exit_code::REPORT_FAILED))?,
            "csv" => {
                let tc = title_column.ok_or((
                    "--title-column required for CSV import.".to_string(),
                    exit_code::INVALID_STATUS_SEVERITY,
                ))?;
                let sc = severity_column.ok_or((
                    "--severity-column required for CSV import.".to_string(),
                    exit_code::INVALID_STATUS_SEVERITY,
                ))?;
                ss_workspace::import::parse_csv(&content, tc, sc)
                    .map_err(|e| (e, exit_code::REPORT_FAILED))?
            }
            _ => {
                return Err((
                    format!("Unknown import format: '{format}'. Use: nessus, csv."),
                    exit_code::INVALID_STATUS_SEVERITY,
                ));
            }
        };
        let summary = ss_workspace::import::import_findings(&ws, path_or_id, parsed)
            .map_err(|e| workspace_error(&e))?;
        println!(
            "Import complete: {} parsed, {} created, {} duplicates skipped.",
            summary.parsed, summary.created, summary.duplicates
        );
        return Ok(());
    }

    // Create new finding
    if let Some(t) = title {
        let path = ss_workspace::findings::create_finding(&ws, path_or_id, t, no_template)
            .map_err(|e| workspace_error(&e))?;
        println!("Created finding: {}", path);
        // Open in $EDITOR
        open_editor(&path)?;
        return Ok(());
    }

    // Update status
    if let Some(s) = status {
        ss_workspace::findings::update_finding_status(&ws, path_or_id, s)
            .map_err(|e| workspace_error(&e))?;
        println!(
            "Updated finding {} status: {}",
            path_or_id,
            color::status(s)
        );
        return Ok(());
    }

    // Update severity
    if let Some(s) = severity {
        ss_workspace::findings::update_finding_severity(&ws, path_or_id, s)
            .map_err(|e| workspace_error(&e))?;
        println!(
            "Updated finding {} severity: {}",
            path_or_id,
            color::severity(s)
        );
        return Ok(());
    }

    // Export
    if let Some(fmt) = export {
        return export_finding(&ws, path_or_id, fmt, to);
    }

    // Show finding
    let content =
        ss_workspace::findings::show_finding(&ws, path_or_id).map_err(|e| workspace_error(&e))?;
    println!("{}", content);
    Ok(())
}

fn export_finding(
    ws: &Workspace,
    path_or_id: &str,
    fmt: &str,
    to: Option<&str>,
) -> Result<(), (String, i32)> {
    let format = ss_workspace::render::OutputFormat::parse_format(fmt)
        .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;

    if path_or_id.contains('/') {
        // Export all findings under a path
        let files = ss_workspace::findings::gather_findings(ws, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        let mut combined = String::new();
        for file in &files {
            let content = std::fs::read_to_string(file).map_err(|e| {
                (
                    format!("Failed to read finding: {}", e),
                    exit_code::REPORT_FAILED,
                )
            })?;
            combined.push_str(&content);
            combined.push_str("\n\n---\n\n");
        }
        return output_rendered(&combined, format, to);
    }

    // Single finding
    let content =
        ss_workspace::findings::show_finding(ws, path_or_id).map_err(|e| workspace_error(&e))?;
    output_rendered(&content, format, to)
}

// ── Requirement commands ──────────────────────────────

fn cmd_req(
    ws_flag: Option<&str>,
    path_or_id: &str,
    title: Option<&str>,
    status: Option<&str>,
    export: Option<&str>,
    to: Option<&str>,
    no_template: bool,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Export
    if let Some(fmt) = export {
        let content = ss_workspace::requirements::show_requirement(&ws, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        return output_rendered(&content, format_enum, to);
    }

    // Create new requirement
    if let Some(t) = title {
        let path = ss_workspace::requirements::create_requirement(&ws, path_or_id, t, no_template)
            .map_err(|e| workspace_error(&e))?;
        println!("Created requirement: {}", path);
        open_editor(&path)?;
        return Ok(());
    }

    // Update status
    if let Some(s) = status {
        ss_workspace::requirements::update_requirement_status(&ws, path_or_id, s)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated requirement {} status: {}", path_or_id, s);
        return Ok(());
    }

    // Show requirement
    let content = ss_workspace::requirements::show_requirement(&ws, path_or_id)
        .map_err(|e| workspace_error(&e))?;
    println!("{}", content);
    Ok(())
}

// ── Scope commands ──────────────────────────────

fn cmd_scope(
    ws_flag: Option<&str>,
    path: &str,
    export: Option<&str>,
    to: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Export scope
    if let Some(fmt) = export {
        let content = ss_workspace::scope::show_scope_content(&ws, path)
            .map_err(|e| workspace_error(&e))?
            .unwrap_or_default();
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        return output_rendered(&content, format_enum, to);
    }

    ss_workspace::scope::open_scope_editor(&ws, path).map_err(|e| workspace_error(&e))?;
    Ok(())
}

// ── Note commands ──────────────────────────────

fn cmd_note(
    ws_flag: Option<&str>,
    path: &str,
    message: Option<&str>,
    export: Option<&str>,
    to: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Export notes
    if let Some(fmt) = export {
        let (eng_dir, _) = ss_workspace::entities::resolve_existing_entity(&ws, path)
            .map_err(|e| workspace_error(&e))?;
        let notes = ss_workspace::notes::list_notes(&eng_dir).map_err(|e| workspace_error(&e))?;
        let mut combined = String::new();
        for note in &notes {
            let content = std::fs::read_to_string(eng_dir.join("notes").join(&note.filename))
                .unwrap_or_default();
            combined.push_str(&content);
            combined.push_str("\n\n");
        }
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        return output_rendered(&combined, format_enum, to);
    }

    // Create note
    let msg = message.ok_or((
        "Note message required (or use --export).".to_string(),
        exit_code::ENTITY_NOT_FOUND,
    ))?;
    let note_path =
        ss_workspace::notes::create_note(&ws, path, msg).map_err(|e| workspace_error(&e))?;
    println!("Created note: {}", note_path);
    Ok(())
}

// ── Report commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_report(
    ws_flag: Option<&str>,
    path: Option<&str>,
    format: Option<&str>,
    template: Option<&str>,
    to: Option<&str>,
    sections: Option<&str>,
    exclude: Option<&str>,
    all: bool,
) -> Result<(), (String, i32)> {
    // Aggregate across all workspaces
    if all {
        let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
        let mut combined_findings = String::new();
        for ws_entry in &global.workspaces {
            if !ws_entry.path.as_std_path().exists() {
                continue;
            }
            if let Ok(ws) = Workspace::load(&ws_entry.path) {
                let files = ss_workspace::findings::gather_findings(&ws, "").unwrap_or_default();
                for file in &files {
                    if let Ok(content) = std::fs::read_to_string(file) {
                        combined_findings.push_str(&content);
                        combined_findings.push_str("\n\n---\n\n");
                    }
                }
            }
        }
        if combined_findings.is_empty() {
            return Err((
                "No findings found across all workspaces.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        let fmt = format.unwrap_or("markdown");
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        let report_md = format!(
            "# Security Assessment Report (All Workspaces)\n\n## Findings\n\n{}",
            combined_findings
        );
        return output_rendered(&report_md, format_enum, to);
    }

    let path = path.ok_or((
        "Path required (or use --all for all workspaces).".to_string(),
        exit_code::ENTITY_NOT_FOUND,
    ))?;
    let ws = resolve_workspace(ws_flag)?;
    let fmt = format.unwrap_or("markdown");
    let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
        .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;

    if fmt == "pdf" && to.is_none() {
        return Err((
            "PDF output requires --to <path>.".to_string(),
            exit_code::REPORT_FAILED,
        ));
    }

    // For PDF: use Typst assembly with sections + metadata
    if fmt == "pdf" {
        let eng_type = read_engagement_type(&ws, path);
        let (target_path, _) = ss_workspace::entities::resolve_existing_entity(&ws, path)
            .map_err(|e| workspace_error(&e))?;

        // Gather findings
        let files =
            ss_workspace::findings::gather_findings(&ws, path).map_err(|e| workspace_error(&e))?;
        let mut findings = Vec::new();
        for file in &files {
            let content = std::fs::read_to_string(file).map_err(|e| {
                (
                    format!("Failed to read finding: {}", e),
                    exit_code::REPORT_FAILED,
                )
            })?;
            let parsed = ss_frontmatter::parse(&content).unwrap_or(ss_frontmatter::Parsed {
                frontmatter: serde_yaml::Value::Null,
                body: content.clone(),
            });
            let id = parsed
                .frontmatter
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let status = parsed
                .frontmatter
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let severity = parsed
                .frontmatter
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let body = ss_workspace::typst_engine::markdown_to_typst(&parsed.body)
                .map_err(|e| (e, exit_code::REPORT_FAILED))?;
            findings.push(ss_workspace::typst_engine::FindingInput {
                id,
                status,
                severity,
                body,
            });
        }

        // Gather sections
        let sections_map = ss_workspace::sections::assemble_sections(
            &ws.root,
            &target_path,
            &eng_type,
            sections,
            exclude,
        )
        .map_err(|e| workspace_error(&e))?;
        let mut typst_sections = std::collections::BTreeMap::new();
        for (name, content) in sections_map {
            let typst_content = ss_workspace::typst_engine::markdown_to_typst(&content)
                .map_err(|e| (e, exit_code::REPORT_FAILED))?;
            typst_sections.insert(name, typst_content);
        }

        // Gather scope
        let scope_content = ss_workspace::scope::show_scope_content(&ws, path)
            .map_err(|e| workspace_error(&e))?
            .map(|s| {
                ss_workspace::typst_engine::markdown_to_typst(&s)
                    .map_err(|e| (e, exit_code::REPORT_FAILED))
            })
            .transpose()?;

        // Extract metadata
        let metadata = extract_metadata(&ws, path);

        // Load template
        let (tmpl_content, tmpl_fn) = if let Some(t) = template {
            let tc = ss_workspace::templates::get_template(&ws, t)
                .map_err(|e| workspace_error(&e))?
                .ok_or((
                    format!("Template '{}' not found.", t),
                    exit_code::REPORT_FAILED,
                ))?;
            (tc, t.to_string())
        } else {
            ss_workspace::typst_engine::load_template(&ws.root, "report")
                .map_err(|e| (e, exit_code::REPORT_FAILED))?
        };

        let inputs = ss_workspace::typst_engine::TypstInputs {
            findings,
            scope: scope_content,
            sections: typst_sections,
            metadata,
            ..Default::default()
        };

        let pdf = ss_workspace::typst_engine::compile_pdf(&tmpl_content, &tmpl_fn, &inputs)
            .map_err(|e| (e, exit_code::REPORT_FAILED))?;
        write_output(&pdf, to)?;
        return Ok(());
    }

    // For Markdown/HTML/JSON: use existing template substitution path
    let files =
        ss_workspace::findings::gather_findings(&ws, path).map_err(|e| workspace_error(&e))?;

    if files.is_empty() {
        return Err((
            format!("No findings found under `{}`.", path),
            exit_code::REPORT_FAILED,
        ));
    }

    // Build report content
    // Template priority: --template flag > project [project.report] > client [client.report] > built-in
    let template_content = if let Some(t) = template {
        ss_workspace::templates::get_template(&ws, t)
    } else {
        let segments: Vec<&str> = path.split('/').collect();
        let client_name = segments.first().copied();
        let project_name = segments.get(1).copied();

        let config_template = client_name.and_then(|c| {
            ss_workspace::entities::get_effective_report_template(&ws, c, project_name)
        });

        if let Some(t) = config_template {
            ss_workspace::templates::get_template(&ws, &t)
        } else {
            ss_workspace::templates::get_template(&ws, "report")
        }
    }
    .map_err(|e| workspace_error(&e))?;

    let report_md = if let Some(ref tmpl) = template_content {
        let mut findings_text = String::new();
        for file in &files {
            let content = std::fs::read_to_string(file).map_err(|e| {
                (
                    format!("Failed to read finding: {}", e),
                    exit_code::REPORT_FAILED,
                )
            })?;
            findings_text.push_str(&content);
            findings_text.push_str("\n\n---\n\n");
        }
        // Include scope.md content if it exists — the user writes engagement
        // info (target, contacts, version, etc.) there, and it flows into reports.
        let scope_content = ss_workspace::scope::show_scope_content(&ws, path)
            .map_err(|e| workspace_error(&e))?
            .unwrap_or_default();
        tmpl.replace("{{findings}}", &findings_text)
            .replace("{{scope}}", &scope_content)
    } else {
        let mut content = String::from("# Security Assessment Report\n\n## Findings\n\n");
        for file in &files {
            let finding = std::fs::read_to_string(file).unwrap_or_default();
            content.push_str(&finding);
            content.push_str("\n\n---\n\n");
        }
        content
    };

    if fmt == "pdf" && to.is_none() {
        return Err((
            "PDF output requires --to <path>.".to_string(),
            exit_code::REPORT_FAILED,
        ));
    }

    output_rendered(&report_md, format_enum, to)
}

// ── SOW commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_sow(
    ws_flag: Option<&str>,
    path: Option<&str>,
    format: Option<&str>,
    template: Option<&str>,
    to: Option<&str>,
    sections: Option<&str>,
    exclude: Option<&str>,
    all: bool,
) -> Result<(), (String, i32)> {
    // Aggregate across all workspaces
    if all {
        let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
        let mut combined_reqs = String::new();
        for ws_entry in &global.workspaces {
            if !ws_entry.path.as_std_path().exists() {
                continue;
            }
            if let Ok(_ws) = Workspace::load(&ws_entry.path) {
                // Walk all clients/projects/engagements
                for client_entry in std::fs::read_dir(&ws_entry.path)
                    .into_iter()
                    .flatten()
                    .flatten()
                {
                    if !client_entry
                        .file_type()
                        .map(|t| t.is_dir())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    let client_dir = client_entry.path();
                    if !client_dir.join("config.toml").exists() {
                        continue;
                    }
                    for proj_entry in std::fs::read_dir(&client_dir)
                        .into_iter()
                        .flatten()
                        .flatten()
                    {
                        if !proj_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            continue;
                        }
                        let proj_dir = proj_entry.path();
                        if !proj_dir.join("config.toml").exists() {
                            continue;
                        }
                        for eng_entry in
                            std::fs::read_dir(&proj_dir).into_iter().flatten().flatten()
                        {
                            if !eng_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                continue;
                            }
                            let eng_dir = eng_entry.path();
                            if !eng_dir.join("config.toml").exists() {
                                continue;
                            }
                            if camino::Utf8Path::from_path(&eng_dir).is_some() {
                                let eng_pathbuf =
                                    camino::Utf8PathBuf::from_path_buf(eng_entry.path())
                                        .unwrap_or_else(|_| camino::Utf8PathBuf::new());
                                if let Ok(reqs) =
                                    ss_workspace::requirements::list_requirements(&eng_pathbuf)
                                {
                                    for req in &reqs {
                                        let content = std::fs::read_to_string(
                                            eng_dir.join("requirements").join(&req.filename),
                                        )
                                        .unwrap_or_default();
                                        combined_reqs.push_str(&content);
                                        combined_reqs.push_str("\n\n");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if combined_reqs.is_empty() {
            return Err((
                "No requirements found across all workspaces.".to_string(),
                exit_code::SOW_FAILED,
            ));
        }
        let fmt = format.unwrap_or("markdown");
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::SOW_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::SOW_FAILED,
            ));
        }
        let sow_md = format!(
            "# Statement of Work (All Workspaces)\n\n## Requirements\n\n{}",
            combined_reqs
        );
        return output_rendered(&sow_md, format_enum, to);
    }

    let path = path.ok_or((
        "Path required (or use --all for all workspaces).".to_string(),
        exit_code::ENTITY_NOT_FOUND,
    ))?;
    let ws = resolve_workspace(ws_flag)?;
    let fmt = format.unwrap_or("markdown");
    let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
        .ok_or((format!("Invalid format: {}", fmt), exit_code::SOW_FAILED))?;

    let (eng_dir, entity_type) = ss_workspace::entities::resolve_existing_entity(&ws, path)
        .map_err(|e| workspace_error(&e))?;
    if entity_type != ss_workspace::entities::EntityType::Engagement {
        return Err((
            "SOW requires an engagement path (client/project/engagement).".to_string(),
            exit_code::SOW_FAILED,
        ));
    }

    if fmt == "pdf" && to.is_none() {
        return Err((
            "PDF output requires --to <path>.".to_string(),
            exit_code::SOW_FAILED,
        ));
    }

    // For PDF: use Typst assembly
    if fmt == "pdf" {
        let eng_type = read_engagement_type(&ws, path);

        // Gather requirements
        let reqs = ss_workspace::requirements::list_requirements(&eng_dir)
            .map_err(|e| workspace_error(&e))?;
        let mut requirements = Vec::new();
        for req in &reqs {
            let content = std::fs::read_to_string(eng_dir.join("requirements").join(&req.filename))
                .unwrap_or_default();
            let parsed = ss_frontmatter::parse(&content).unwrap_or(ss_frontmatter::Parsed {
                frontmatter: serde_yaml::Value::Null,
                body: content.clone(),
            });
            let id = parsed
                .frontmatter
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let status = parsed
                .frontmatter
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let body = ss_workspace::typst_engine::markdown_to_typst(&parsed.body)
                .map_err(|e| (e, exit_code::SOW_FAILED))?;
            requirements.push(ss_workspace::typst_engine::RequirementInput { id, status, body });
        }

        // Gather sections
        let sections_map = ss_workspace::sections::assemble_sections(
            &ws.root, &eng_dir, &eng_type, sections, exclude,
        )
        .map_err(|e| workspace_error(&e))?;
        let mut typst_sections = std::collections::BTreeMap::new();
        for (name, content) in sections_map {
            let typst_content = ss_workspace::typst_engine::markdown_to_typst(&content)
                .map_err(|e| (e, exit_code::SOW_FAILED))?;
            typst_sections.insert(name, typst_content);
        }

        // Gather scope
        let scope_content = ss_workspace::scope::show_scope_content(&ws, path)
            .map_err(|e| workspace_error(&e))?
            .map(|s| {
                ss_workspace::typst_engine::markdown_to_typst(&s)
                    .map_err(|e| (e, exit_code::SOW_FAILED))
            })
            .transpose()?;

        // Extract metadata
        let metadata = extract_metadata(&ws, path);

        // Load template
        let (tmpl_content, tmpl_fn) = if let Some(t) = template {
            let tc = ss_workspace::templates::get_template(&ws, t)
                .map_err(|e| workspace_error(&e))?
                .ok_or((
                    format!("Template '{}' not found.", t),
                    exit_code::SOW_FAILED,
                ))?;
            (tc, t.to_string())
        } else {
            ss_workspace::typst_engine::load_template(&ws.root, "sow")
                .map_err(|e| (e, exit_code::SOW_FAILED))?
        };

        let inputs = ss_workspace::typst_engine::TypstInputs {
            requirements,
            scope: scope_content,
            sections: typst_sections,
            metadata,
            ..Default::default()
        };

        let pdf = ss_workspace::typst_engine::compile_pdf(&tmpl_content, &tmpl_fn, &inputs)
            .map_err(|e| (e, exit_code::SOW_FAILED))?;
        write_output(&pdf, to)?;
        return Ok(());
    }

    // For Markdown/HTML/JSON: use existing template substitution path

    // Gather requirements
    let reqs =
        ss_workspace::requirements::list_requirements(&eng_dir).map_err(|e| workspace_error(&e))?;

    // Get scope content
    let scope_content = ss_workspace::scope::show_scope_content(&ws, path)
        .map_err(|e| workspace_error(&e))?
        .unwrap_or_default();

    // Build SOW from template
    // Template priority: --template flag > project [project.sow] > client [client.sow] > built-in
    let template_content = if let Some(t) = template {
        ss_workspace::templates::get_template(&ws, t)
    } else {
        let segments: Vec<&str> = path.split('/').collect();
        let client_name = segments.first().copied();
        let project_name = segments.get(1).copied();

        let config_template = client_name
            .and_then(|c| ss_workspace::entities::get_effective_sow_template(&ws, c, project_name));

        if let Some(t) = config_template {
            ss_workspace::templates::get_template(&ws, &t)
        } else {
            ss_workspace::templates::get_template(&ws, "sow")
        }
    }
    .map_err(|e| workspace_error(&e))?;

    let mut reqs_text = String::new();
    for req in &reqs {
        let content = std::fs::read_to_string(eng_dir.join("requirements").join(&req.filename))
            .map_err(|e| {
                (
                    format!("Failed to read requirement: {}", e),
                    exit_code::SOW_FAILED,
                )
            })?;
        reqs_text.push_str(&content);
        reqs_text.push_str("\n\n");
    }

    let sow_md = if let Some(ref tmpl) = template_content {
        tmpl.replace("{{requirements}}", &reqs_text)
            .replace("{{scope}}", &scope_content)
    } else {
        format!(
            "# Statement of Work\n\n## Scope\n\n{}\n\n## Requirements\n\n{}\n",
            scope_content, reqs_text
        )
    };

    if fmt == "pdf" && to.is_none() {
        return Err((
            "PDF output requires --to <path>.".to_string(),
            exit_code::SOW_FAILED,
        ));
    }

    output_rendered(&sow_md, format_enum, to)
}

// ── Helper functions for Typst assembly ──────────────────────────────

fn read_engagement_type(ws: &Workspace, path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() >= 3 {
        let eng_config = ws
            .root
            .join(segments[0])
            .join(segments[1])
            .join(segments[2])
            .join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&eng_config) {
            if let Ok(toml_val) = content.parse::<toml::Value>() {
                if let Some(t) = toml_val
                    .get("engagement")
                    .and_then(|e| e.get("type"))
                    .and_then(|t| t.as_str())
                {
                    return t.to_string();
                }
            }
        }
    }
    "assessment".to_string()
}

fn extract_metadata(ws: &Workspace, path: &str) -> std::collections::BTreeMap<String, String> {
    let mut meta = std::collections::BTreeMap::new();
    let segments: Vec<&str> = path.split('/').collect();

    // Client info
    if let Some(client) = segments.first() {
        meta.insert("client_name".to_string(), client.to_string());
        let client_config = ws.root.join(client).join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&client_config) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(client) = val.get("client") {
                    if let Some(email) = client.get("email").and_then(|v| v.as_str()) {
                        if !email.is_empty() {
                            meta.insert("client_email".to_string(), email.to_string());
                        }
                    }
                    if let Some(phone) = client.get("phone").and_then(|v| v.as_str()) {
                        if !phone.is_empty() {
                            meta.insert("client_phone".to_string(), phone.to_string());
                        }
                    }
                }
            }
        }
    }

    // Project info
    if let Some(project) = segments.get(1) {
        meta.insert("project_name".to_string(), project.to_string());
        let project_config = ws.root.join(segments[0]).join(project).join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&project_config) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(proj) = val.get("project") {
                    if let Some(start) = proj.get("start_date").and_then(|v| v.as_str()) {
                        if !start.is_empty() {
                            meta.insert("start_date".to_string(), start.to_string());
                        }
                    }
                    if let Some(end) = proj.get("end_date").and_then(|v| v.as_str()) {
                        if !end.is_empty() {
                            meta.insert("end_date".to_string(), end.to_string());
                        }
                    }
                }
            }
        }
    }

    // Engagement info
    if let Some(eng) = segments.get(2) {
        meta.insert("engagement_name".to_string(), eng.to_string());
        let eng_config = ws
            .root
            .join(segments[0])
            .join(segments[1])
            .join(eng)
            .join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&eng_config) {
            if let Ok(val) = content.parse::<toml::Value>() {
                if let Some(eng) = val.get("engagement") {
                    if let Some(t) = eng.get("type").and_then(|v| v.as_str()) {
                        meta.insert("engagement_type".to_string(), t.to_string());
                    }
                    if let Some(start) = eng.get("start_date").and_then(|v| v.as_str()) {
                        if !start.is_empty() {
                            meta.insert("start_date".to_string(), start.to_string());
                        }
                    }
                    if let Some(end) = eng.get("end_date").and_then(|v| v.as_str()) {
                        if !end.is_empty() {
                            meta.insert("end_date".to_string(), end.to_string());
                        }
                    }
                }
            }
        }
    }

    meta
}

fn write_output(data: &[u8], to: Option<&str>) -> Result<(), (String, i32)> {
    if let Some(path) = to {
        std::fs::write(path, data).map_err(|e| {
            (
                format!("Cannot write to '{}': {}", path, e),
                exit_code::REPORT_FAILED,
            )
        })?;
        println!("Written to {}", path);
    } else {
        std::io::Write::write_all(&mut std::io::stdout(), data).map_err(|e| {
            (
                format!("Failed to write to stdout: {}", e),
                exit_code::REPORT_FAILED,
            )
        })?;
    }
    Ok(())
}

// ── Checklist commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_checklist(
    ws_flag: Option<&str>,
    path: &str,
    assign: Option<&str>,
    list: bool,
    item: Option<&str>,
    status: Option<&str>,
    finding: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    if let Some(name) = assign {
        let count = ss_workspace::checklists::assign_checklist(&ws, path, name)
            .map_err(|e| workspace_error(&e))?;
        println!("Assigned checklist '{}': {} items.", name, count);
        return Ok(());
    }

    if list {
        let tracking =
            ss_workspace::checklists::load_tracking(&ws, path).map_err(|e| workspace_error(&e))?;
        let coverage = ss_workspace::checklists::compute_coverage(&tracking);
        println!(
            "Checklist: {} ({:.0}% coverage)",
            tracking.tracking.checklist_name, coverage
        );
        println!();
        for item in &tracking.items {
            let finding = if item.finding_id.is_empty() {
                ""
            } else {
                &format!(" → {}", item.finding_id)
            };
            println!("{}\t{}\t{}", item.id, item.status, finding);
        }
        return Ok(());
    }

    if let Some(item_id) = item {
        if let Some(s) = status {
            ss_workspace::checklists::update_item_status(&ws, path, item_id, s)
                .map_err(|e| workspace_error(&e))?;
            println!("Updated {} status: {}", item_id, s);
            return Ok(());
        }
        if let Some(f) = finding {
            ss_workspace::checklists::link_finding(&ws, path, item_id, f)
                .map_err(|e| workspace_error(&e))?;
            println!("Linked {} → {}", item_id, f);
            return Ok(());
        }
    }

    println!(
        "Usage: sm checklist <path> --assign <NAME> | --list | --item <ID> --status <STATUS> | --item <ID> --finding <FINDING_ID>"
    );
    Ok(())
}

// ── Search commands ──────────────────────────────

fn cmd_search(
    ws_flag: Option<&str>,
    query: &str,
    entity_type: Option<&str>,
    client: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;
    let results = ss_workspace::search::search_workspace(&ws.root, query, entity_type, client)
        .map_err(|e| workspace_error(&e))?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    // Group by entity type
    let mut current_type = String::new();
    for r in &results {
        if r.entity_type != current_type {
            current_type = r.entity_type.clone();
            println!(
                "
{}:",
                current_type
            );
        }
        println!("  {}:{}: {}", r.file_path, r.line_number, r.matching_line);
    }
    println!(
        "
{} result(s) found.",
        results.len()
    );
    Ok(())
}

// ── Credential commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_credential(
    ws_flag: Option<&str>,
    path_or_id: &str,
    add: bool,
    label: Option<&str>,
    cred_type: Option<&str>,
    list: bool,
    show: bool,
    status: Option<&str>,
    rm: bool,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Prompt for master password
    let password = rpassword::prompt_password("Master password: ").map_err(|e| {
        (
            format!("Password prompt failed: {e}"),
            exit_code::INVALID_TOML,
        )
    })?;

    if add {
        let label = label.ok_or((
            "--label is required for add.".to_string(),
            exit_code::MISSING_REQUIRED_FIELD,
        ))?;
        let cred_type = cred_type.ok_or((
            "--type is required for add.".to_string(),
            exit_code::MISSING_REQUIRED_FIELD,
        ))?;
        let value = rpassword::prompt_password("Credential value: ").map_err(|e| {
            (
                format!("Password prompt failed: {e}"),
                exit_code::INVALID_TOML,
            )
        })?;
        let id = ss_workspace::credentials::add_credential(
            &ws.root, &password, path_or_id, label, cred_type, &value, "",
        )
        .map_err(|e| workspace_error(&e))?;
        println!("Added credential: {}", id);
        return Ok(());
    }

    if list {
        let creds = ss_workspace::credentials::list_credentials(&ws.root, &password, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        if creds.is_empty() {
            println!("No credentials found.");
        } else {
            for c in creds {
                println!("{}\t{}\t{}\t{}", c.id, c.label, c.cred_type, c.status);
            }
        }
        return Ok(());
    }

    if show {
        let cred = ss_workspace::credentials::show_credential(&ws.root, &password, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        println!("ID:     {}", cred.id);
        println!("Label:  {}", cred.label);
        println!("Type:   {}", cred.cred_type);
        println!("Status: {}", cred.status);
        println!("Value:  {}", cred.value);
        if !cred.notes.is_empty() {
            println!("Notes:  {}", cred.notes);
        }
        return Ok(());
    }

    if let Some(s) = status {
        ss_workspace::credentials::update_credential_status(&ws.root, &password, path_or_id, s)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated credential {} status: {}", path_or_id, s);
        return Ok(());
    }

    if rm {
        ss_workspace::credentials::remove_credential(&ws.root, &password, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        println!("Removed credential: {}", path_or_id);
        return Ok(());
    }

    println!(
        "Usage: sm credential <path> --add --label <L> --type <T> | --list | <ID> --show | <ID> --status <S> | <ID> --rm"
    );
    Ok(())
}

// ── Evidence commands ──────────────────────────────

#[allow(clippy::needless_borrow)]
fn cmd_evidence(
    ws_flag: Option<&str>,
    path: &str,
    add: Option<&str>,
    list: bool,
    show: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;
    let (eng_dir, entity_type) = ss_workspace::entities::resolve_existing_entity(&ws, path)
        .map_err(|e| workspace_error(&e))?;
    if entity_type != ss_workspace::entities::EntityType::Engagement {
        return Err((
            "Evidence requires an engagement path (client/project/engagement).".to_string(),
            exit_code::ENTITY_NOT_FOUND,
        ));
    }

    if let Some(file) = add {
        let source = camino::Utf8PathBuf::from(file);
        let filename = ss_workspace::evidence::add_evidence(&ws.root, &eng_dir, &source)
            .map_err(|e| workspace_error(&e))?;
        println!("Added evidence: {}", filename);
        println!(
            "Note: Evidence is stored in plaintext. Use OS-level disk encryption for sensitive files."
        );
        return Ok(());
    }

    if list {
        let entries =
            ss_workspace::evidence::list_evidence(&eng_dir).map_err(|e| workspace_error(&e))?;
        if entries.is_empty() {
            println!("No evidence files found.");
        } else {
            for entry in entries {
                println!(
                    "{}\t{}\t{}\t{}",
                    entry.filename, entry.size, entry.sha256, entry.date_added
                );
            }
        }
        return Ok(());
    }

    if let Some(filename) = show {
        let file_path = eng_dir.join("evidence").join(filename);
        if !file_path.exists() {
            return Err((
                format!("Evidence file '{}' not found.", filename),
                exit_code::ENTITY_NOT_FOUND,
            ));
        }
        ss_workspace::check_symlink_escape(&ws.root, &file_path)
            .map_err(|e| workspace_error(&e))?;
        open_editor(&file_path)?;
        return Ok(());
    }

    // No flags — show help
    println!("Usage: sm evidence <path> --add <FILE> | --list | --show <FILENAME>");
    Ok(())
}

// ── Engagement commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_engagement(
    ws_flag: Option<&str>,
    path: &str,
    status: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    credentials_ready: bool,
    retest: bool,
    from: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Create retest engagement
    if retest {
        let original = from.ok_or((
            "--from <original_engagement> is required for --retest.".to_string(),
            exit_code::MISSING_REQUIRED_FIELD,
        ))?;
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() != 3 {
            return Err((
                "Retest path must be client/project/retest_name.".to_string(),
                exit_code::ENTITY_NOT_FOUND,
            ));
        }
        let copied = ss_workspace::entities::create_retest_engagement(
            &ws,
            segments[0],
            segments[1],
            segments[2],
            original,
        )
        .map_err(|e| workspace_error(&e))?;
        println!(
            "Created retest engagement: {} (copied {} findings from {})",
            path, copied, original
        );
        return Ok(());
    }

    if credentials_ready {
        let new_val = ss_workspace::entities::toggle_credentials_ready(&ws, path)
            .map_err(|e| workspace_error(&e))?;
        println!("Credentials ready: {}", new_val);
        return Ok(());
    }

    if let Some(s) = status {
        ss_workspace::entities::update_engagement_status(&ws, path, s)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated engagement {} status: {}", path, s);
        return Ok(());
    }

    if let Some(d) = start_date {
        ss_workspace::entities::update_engagement_date(&ws, path, "start_date", d)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated engagement {} start date: {}", path, d);
        return Ok(());
    }

    if let Some(d) = end_date {
        ss_workspace::entities::update_engagement_date(&ws, path, "end_date", d)
            .map_err(|e| workspace_error(&e))?;
        println!("Updated engagement {} end date: {}", path, d);
        return Ok(());
    }

    // No flags — show current engagement config
    let (eng_dir, _) = ss_workspace::entities::resolve_existing_entity(&ws, path)
        .map_err(|e| workspace_error(&e))?;
    let config_path = eng_dir.join("config.toml");
    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        (
            format!("Failed to read config: {}", e),
            exit_code::INVALID_TOML,
        )
    })?;
    println!("{}", content);
    Ok(())
}

// ── Document commands ──────────────────────────────

#[allow(clippy::too_many_arguments)]
fn cmd_document(
    ws_flag: Option<&str>,
    path_or_id: &str,
    title: Option<&str>,
    doc_type: Option<&str>,
    finalize: bool,
    unlock: bool,
    export: Option<&str>,
    to: Option<&str>,
) -> Result<(), (String, i32)> {
    let ws = resolve_workspace(ws_flag)?;

    // Create new document
    if let Some(t) = title {
        let dt = doc_type.ok_or((
            "--type is required for document creation".to_string(),
            exit_code::INVALID_STATUS_SEVERITY,
        ))?;
        let dt_enum = ss_workspace::documents::DocumentType::parse(dt).ok_or((
            format!(
                "'{}' is not a valid document type. Use: roe, nda, proposal, custom.",
                dt
            ),
            exit_code::INVALID_STATUS_SEVERITY,
        ))?;
        let (parent_path, _) = ss_workspace::entities::resolve_existing_entity(&ws, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        let doc =
            ss_workspace::documents::create_document(&ws.root, &parent_path, t, dt_enum, None)
                .map_err(|e| workspace_error(&e))?;
        println!("Created document: {}", doc.path);
        open_editor(&doc.path)?;
        return Ok(());
    }

    // Finalize
    if finalize {
        ss_workspace::documents::finalize_document(&ws.root, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        println!("Finalized document {}.", path_or_id);
        return Ok(());
    }

    // Unlock
    if unlock {
        ss_workspace::documents::unlock_document(&ws.root, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        println!("Unlocked document {}.", path_or_id);
        return Ok(());
    }

    // Export
    if let Some(fmt) = export {
        let doc = ss_workspace::documents::find_document_by_id(&ws.root, path_or_id)
            .map_err(|e| workspace_error(&e))?;
        let format_enum = ss_workspace::render::OutputFormat::parse_format(fmt)
            .ok_or((format!("Invalid format: {}", fmt), exit_code::REPORT_FAILED))?;
        if fmt == "pdf" && to.is_none() {
            return Err((
                "PDF output requires --to <path>.".to_string(),
                exit_code::REPORT_FAILED,
            ));
        }
        let content = format!(
            "---\nid: \"{}\"\ntype: \"{}\"\nstatus: \"{}\"\n---\n\n{}",
            doc.frontmatter.id, doc.frontmatter.doc_type, doc.frontmatter.status, doc.body
        );
        return output_rendered(&content, format_enum, to);
    }

    // Show document
    let doc = ss_workspace::documents::find_document_by_id(&ws.root, path_or_id)
        .map_err(|e| workspace_error(&e))?;
    println!("---");
    println!("id: \"{}\"", doc.frontmatter.id);
    println!("type: \"{}\"", doc.frontmatter.doc_type);
    println!("status: \"{}\"", doc.frontmatter.status);
    println!("---\n");
    println!("{}", doc.body);
    Ok(())
}

// ── Stats commands ──────────────────────────────

fn cmd_stats(ws_flag: Option<&str>, client: Option<&str>, all: bool) -> Result<(), (String, i32)> {
    if all {
        // Aggregate across all known workspaces
        let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
        let mut total = ss_workspace::stats::Stats::default();
        for ws_entry in &global.workspaces {
            if !ws_entry.path.as_std_path().exists() {
                continue;
            }
            if let Ok(ws) = Workspace::load(&ws_entry.path) {
                let stats = ss_workspace::stats::compute_workspace_stats(&ws);
                total.clients += stats.clients;
                total.projects += stats.projects;
                total.engagements += stats.engagements;
                total.findings_total += stats.findings_total;
                total.findings_open += stats.findings_open;
                for (sev, count) in stats.findings_by_severity {
                    if let Some(existing) = total
                        .findings_by_severity
                        .iter_mut()
                        .find(|(s, _)| s == &sev)
                    {
                        existing.1 += count;
                    } else {
                        total.findings_by_severity.push((sev, count));
                    }
                }
            }
        }
        print_stats("All workspaces", &total);
        return Ok(());
    }

    let ws = resolve_workspace(ws_flag)?;

    if let Some(client_name) = client {
        let stats = ss_workspace::stats::compute_client_stats(&ws, client_name);
        if stats.clients == 0 {
            return Err((
                format!("No client matching `{}` found.", client_name),
                exit_code::ENTITY_NOT_FOUND,
            ));
        }
        print_stats(client_name, &stats);
        return Ok(());
    }

    let stats = ss_workspace::stats::compute_workspace_stats(&ws);
    print_stats(&ws.config.workspace.name, &stats);
    Ok(())
}

fn print_stats(label: &str, stats: &ss_workspace::stats::Stats) {
    println!("Stats: {}", label);
    println!("  Clients:     {}", stats.clients);
    println!("  Projects:    {}", stats.projects);
    println!("  Engagements: {}", stats.engagements);
    println!(
        "  Findings:    {} ({} open)",
        stats.findings_total, stats.findings_open
    );
    if !stats.findings_by_severity.is_empty() {
        println!("  Findings by severity:");
        for (sev, count) in &stats.findings_by_severity {
            println!("    {}: {}", color::severity(sev), count);
        }
    }
    if !stats.engagements_by_status.is_empty() {
        println!("  Engagements by status:");
        for (status, count) in &stats.engagements_by_status {
            println!("    {}: {}", status, count);
        }
    }
    if !stats.clients_by_priority.is_empty() {
        println!("  Clients by priority:");
        for (pri, count) in &stats.clients_by_priority {
            println!("    {}: {}", pri, count);
        }
    }
    if !stats.projects_by_priority.is_empty() {
        println!("  Projects by priority:");
        for (pri, count) in &stats.projects_by_priority {
            println!("    {}: {}", pri, count);
        }
    }
    if !stats.open_findings_per_project.is_empty() {
        println!("  Open findings per project:");
        for (proj, count) in &stats.open_findings_per_project {
            println!("    {}: {}", proj, count);
        }
    }
}

// ── Template commands ──────────────────────────────

fn create_template(ws: &Workspace, name: &str) -> Result<(), (String, i32)> {
    let path =
        ss_workspace::templates::create_template(ws, name).map_err(|e| workspace_error(&e))?;
    println!("Created template: {}", path);
    Ok(())
}

// ── Helpers ──────────────────────────────

fn resolve_workspace(ws_flag: Option<&str>) -> Result<Workspace, (String, i32)> {
    // Find workspace root without parsing config (may be old format).
    let root = Workspace::resolve_root(ws_flag).map_err(|e| workspace_error(&e))?;

    // Scan all config files — pure read, no modifications.
    let report =
        ss_workspace::entities::scan_all_configs(&root).map_err(|e| workspace_error(&e))?;

    // Corrupt configs block everything — tell the user, don't skip.
    if !report.corrupt.is_empty() {
        eprintln!("error: The following config files are corrupt and cannot be parsed:");
        for path in &report.corrupt {
            eprintln!("  {}", path);
        }
        eprintln!("Fix or remove these files, then try again.");
        return Err((
            "Corrupt config files found. Migration cannot proceed.".to_string(),
            exit_code::INVALID_TOML,
        ));
    }

    // Configs need migration — prompt user before modifying files.
    if !report.needs_migration.is_empty() {
        eprintln!(
            "{} config file(s) need migration to the current format:",
            report.needs_migration.len()
        );
        for path in &report.needs_migration {
            eprintln!("  {}", path);
        }
        eprintln!("Migration fills in new fields with defaults. Existing data is preserved.");
        eprint!("Proceed with migration? [y/N] ");
        if !read_yes_no() {
            return Err((
                "Migration declined. Run the command again and confirm to migrate.".to_string(),
                exit_code::INVALID_TOML,
            ));
        }
        let count = ss_workspace::entities::migrate_all(&root).map_err(|e| workspace_error(&e))?;
        eprintln!("Migrated {} config file(s).", count);
    }

    // Now load the workspace — all configs are current format.
    Workspace::load(&root).map_err(|e| workspace_error(&e))
}

fn open_editor(path: &camino::Utf8PathBuf) -> Result<(), (String, i32)> {
    ss_workspace::spawn_editor(path).map_err(|e| workspace_error(&e))?;

    // Validate frontmatter after editor exits
    if path.extension() == Some("md") {
        match ss_frontmatter::parse_file(path.as_std_path()) {
            Ok(parsed) => {
                if parsed.has_frontmatter() {
                    // Check for required fields based on file location
                    let fm = &parsed.frontmatter;
                    if fm.get("id").is_none() {
                        eprintln!(
                            "warning: {} is missing frontmatter field 'id'",
                            path.file_name().unwrap_or("file")
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "warning: {} has invalid frontmatter: {}",
                    path.file_name().unwrap_or("file"),
                    e
                );
            }
        }
    }
    Ok(())
}

fn output_rendered(
    content: &str,
    format: ss_workspace::render::OutputFormat,
    to: Option<&str>,
) -> Result<(), (String, i32)> {
    let rendered = ss_workspace::render::render(content, format)
        .map_err(|e| (format!("Render failed: {}", e), exit_code::REPORT_FAILED))?;

    if let Some(path) = to {
        let path = ss_workspace::expand_tilde(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| (format!("Failed to create directory: {}", e), 1))?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &rendered).map_err(|e| (format!("Failed to write: {}", e), 1))?;
        std::fs::rename(&tmp, &path).map_err(|e| (format!("Failed to write: {}", e), 1))?;
        println!("Written to: {}", path);
    } else {
        use std::io::Write;
        std::io::stdout()
            .write_all(&rendered)
            .map_err(|e| (format!("Failed to write: {}", e), 1))?;
    }
    Ok(())
}

/// Print a confirmation warning before removal.
fn print_confirmation(path: &str) {
    eprintln!(
        "Warning: This will move `{}` and all files inside it to trash. Proceed? [y/N] ",
        path
    );
}

/// Read a y/N response from stdin. Returns true for y/Y, false otherwise.
fn read_yes_no() -> bool {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Print the removal result based on the method used.
fn print_removal(kind: &str, name: &str, method: ss_workspace::RemovalMethod) {
    use ss_workspace::RemovalMethod;
    let label = if kind.is_empty() {
        name.to_string()
    } else {
        format!("{}: {}", kind, name)
    };
    match method {
        RemovalMethod::Trashed => println!("Moved to trash: {}", label),
        RemovalMethod::PermanentlyDeleted => println!("Removed: {}", label),
    }
}
