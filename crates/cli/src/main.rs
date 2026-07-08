use clap::Parser;
use ss_workspace::{Workspace, global::GlobalConfig};

mod cli_def;
mod error;

use cli_def::{Cli, Commands, ConfigAction};
use error::{exit_code, fail, workspace_error};

fn main() {
    let cli = Cli::parse();

    let result = run(cli);
    if let Err((msg, code)) = result {
        fail(&msg, code);
    }
}

fn run(cli: Cli) -> Result<(), (String, i32)> {
    match cli.command {
        Commands::New { path } => cmd_new(path, cli.workspace.as_deref()),
        Commands::Status => cmd_status(cli.workspace.as_deref()),
        Commands::Config { action } => cmd_config(action),
        Commands::Check { fix } => cmd_check(fix, cli.workspace.as_deref()),
        Commands::Ls {
            path,
            findings,
            requirements,
            notes,
            scope,
            severity,
            status,
        } => {
            let _ = (&findings, &requirements, &notes, &scope, &severity, &status);
            stub("ls", path.as_deref())
        }
        Commands::Show { path } => stub("show", Some(&path)),
        Commands::Edit { path } => stub("edit", Some(&path)),
        Commands::Rm { path, force } => {
            if !force {
                return Err((
                    format!(
                        "Removing `{}` requires `--force`. Re-run with `--force` to confirm.",
                        path
                    ),
                    exit_code::FORCE_FLAG_MISSING,
                ));
            }
            stub("rm", Some(&path))
        }
        Commands::Stats { client, all } => {
            let _ = (client, all);
            stub_no_path("stats")
        }
        Commands::Finding {
            path_or_id,
            title,
            status,
            severity,
            export,
            to,
            no_template,
        } => {
            let _ = (title, status, severity, export, to, no_template);
            stub("finding", Some(&path_or_id))
        }
        Commands::Req {
            path_or_id,
            title,
            status,
            no_template,
        } => {
            let _ = (title, status, no_template);
            stub("req", Some(&path_or_id))
        }
        Commands::Scope { path } => stub("scope", Some(&path)),
        Commands::Note { path, message } => {
            let _ = message;
            stub("note", Some(&path))
        }
        Commands::Report {
            path,
            format,
            template,
            to,
        } => {
            let _ = (format, template, to);
            stub("report", Some(&path))
        }
        Commands::Sow {
            path,
            format,
            template,
            to,
        } => {
            let _ = (format, template, to);
            stub("sow", Some(&path))
        }
    }
}

/// Create a workspace or entity.
fn cmd_new(path: Option<String>, _workspace: Option<&str>) -> Result<(), (String, i32)> {
    match path {
        None => {
            // sm new — create workspace in current directory
            let cwd = ss_workspace::current_dir().map_err(|e| workspace_error(&e))?;
            create_workspace(&cwd)
        }
        Some(p) if ss_workspace::is_absolute_path(&p) => {
            // sm new <absolute_path> — create workspace at path
            let target = ss_workspace::expand_tilde(&p);
            create_workspace(&target)
        }
        Some(p) => {
            // sm new <name> or <client>/<name> etc — entity creation (slice 2+)
            let segments: Vec<&str> = p.split('/').collect();
            match segments.len() {
                1 => {
                    // Could be a client name or a workspace name under default workspace
                    // For now, treat single name as workspace under default workspace
                    // Full client creation is in slice 2
                    let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;
                    let default = global
                        .default_workspace()
                        .ok_or((
                            "No default workspace configured. Set one with `sm config set default_workspace <path>`."
                                .to_string(),
                            exit_code::NOT_A_WORKSPACE,
                        ))?;
                    let target = default.join(segments[0]);
                    create_workspace(&target)
                }
                _ => {
                    // Multi-segment — hierarchy creation (slice 2+)
                    stub("new entity", Some(&p))
                }
            }
        }
    }
}

/// Create a workspace and register it in the global config.
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

/// Show current workspace info.
fn cmd_status(workspace: Option<&str>) -> Result<(), (String, i32)> {
    let ws = Workspace::resolve(workspace).map_err(|e| workspace_error(&e))?;
    println!("Workspace: {}", ws.root);
    println!("Name: {}", ws.config.workspace.name);
    println!("Created: {}", ws.config.workspace.created);
    Ok(())
}

/// Show or set global configuration.
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
                println!(
                    "Set default_workspace to {}",
                    global.default_workspace().unwrap()
                );
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

/// Check workspace health.
fn cmd_check(fix: bool, workspace: Option<&str>) -> Result<(), (String, i32)> {
    let global = GlobalConfig::load().map_err(|e| workspace_error(&e))?;

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

    // Try to resolve the current workspace
    if let Ok(ws) = Workspace::resolve(workspace) {
        println!(
            "Current workspace: {} ({})",
            ws.config.workspace.name, ws.root
        );
    }

    Ok(())
}

/// Stub for not-yet-implemented commands.
fn stub(cmd: &str, path: Option<&str>) -> Result<(), (String, i32)> {
    if let Some(p) = path {
        println!("{}: {} (not implemented yet)", cmd, p);
    } else {
        println!("{} (not implemented yet)", cmd);
    }
    Ok(())
}

fn stub_no_path(cmd: &str) -> Result<(), (String, i32)> {
    println!("{} (not implemented yet)", cmd);
    Ok(())
}
