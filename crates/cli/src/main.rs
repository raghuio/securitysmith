use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use ss_workspace::{Client, GlobalConfig, Workspace, WorkspaceError};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sm")]
#[command(about = "A CLI for managing penetration testing engagements")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new workspace
    #[command(visible_alias = "n")]
    New {
        /// Exact path where the workspace should be created
        path: Option<Utf8PathBuf>,

        /// Name of the workspace; creates it under the configured default root
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Show the current workspace
    #[command(visible_alias = "h")]
    Here,

    /// Manage clients
    #[command(visible_alias = "c")]
    Client {
        #[command(subcommand)]
        action: ClientAction,
    },

    /// Manage global configuration
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
enum ClientAction {
    /// Add a new client to the current workspace
    #[command(visible_alias = "a")]
    Add {
        /// Short name used for directories and commands
        #[arg(short, long)]
        short: String,
        /// Full display name of the client
        #[arg(short, long)]
        display: String,
    },
    /// List clients in the current workspace
    #[command(visible_alias = "l")]
    List,
    /// Remove a client and all its files
    #[command(visible_alias = "r")]
    Rm {
        /// Short name of the client to remove
        short: String,
    },
    /// Rename a client
    #[command(visible_alias = "re")]
    Rename {
        /// Current short name
        old: String,
        /// New short name
        new: String,
    },
    /// Move a client to another workspace
    #[command(visible_alias = "m")]
    Move {
        /// Short name of the client to move
        short: String,
        /// Path or name of the target workspace
        #[arg(short, long)]
        to: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show the current global configuration
    #[command(visible_alias = "s")]
    Show,
    /// Set a global configuration value
    #[command(visible_alias = "se")]
    Set {
        /// Setting name (e.g., default_workspace_root)
        key: String,
        /// Setting value
        value: String,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), WorkspaceError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { path, name } => {
            let target = resolve_new_target(path, name)?;
            let ws = Workspace::init(&target)?;

            let mut global = GlobalConfig::load()?;
            let ws_name = ws.config.workspace.name.clone();
            global.register_workspace(&ws_name, &ws.root);
            global.save()?;

            println!("Created workspace at {}", ws.root);
        }
        Commands::Here => {
            let cwd = current_dir()?;
            let ws = Workspace::find(&cwd)?;
            println!("Workspace: {}", ws.root);
            println!("Name: {}", ws.config.workspace.name);
        }
        Commands::Client { action } => {
            let cwd = current_dir()?;
            let ws = Workspace::find(&cwd)?;
            match action {
                ClientAction::Add { short, display } => {
                    let client = Client::new(&short, &display);
                    client.create(&ws)?;
                    println!("Created client {} at {}", short, client.dir(&ws));
                }
                ClientAction::List => {
                    let clients = Client::list(&ws)?;
                    if clients.is_empty() {
                        println!("No clients in workspace.");
                    } else {
                        for client in clients {
                            println!("{} - {}", client.short_name, client.display_name);
                        }
                    }
                }
                ClientAction::Rm { short } => {
                    Client::remove(&ws, &short)?;
                    println!("Removed client {}", short);
                }
                ClientAction::Rename { old, new } => {
                    Client::rename(&ws, &old, &new)?;
                    println!("Renamed client {} to {}", old, new);
                }
                ClientAction::Move { short, to } => {
                    let target_ws = resolve_target_workspace(&ws, &to)?;
                    Client::move_to_workspace(&ws, &short, &target_ws)?;
                    println!("Moved client {} to {}", short, target_ws.root);
                }
            }
        }
        Commands::Config { action } => {
            let mut global = GlobalConfig::load()?;
            match action {
                None | Some(ConfigAction::Show) => {
                    println!("config file: {}", ss_workspace::global::config_path()?);
                    match global.default_root() {
                        Some(root) => println!("default_workspace_root: {}", root),
                        None => println!("default_workspace_root: not set"),
                    }
                    println!("known workspaces:");
                    for ws in &global.workspaces {
                        println!("  {} = {}", ws.name, ws.path);
                    }
                }
                Some(ConfigAction::Set { key, value }) => {
                    match key.as_str() {
                        "default_workspace_root" => {
                            let expanded = expand_tilde(&value);
                            let path = Utf8PathBuf::from_path_buf(expanded)
                                .map_err(|p| WorkspaceError::Io(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("path is not valid UTF-8: {:?}", p),
                                )))?;
                            global.set_default_root(path);
                            global.save()?;
                            println!("Set default_workspace_root to {}", global.default_root().unwrap());
                        }
                        _ => {
                            eprintln!("error: unknown config key: {}", key);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn resolve_target_workspace(
    _source_ws: &Workspace,
    target: &str,
) -> Result<Workspace, WorkspaceError> {
    // If target is an absolute or relative path that exists as a workspace, use it.
    if let Ok(target_path) = Utf8PathBuf::from_path_buf(expand_tilde(target)) {
        if target_path.join(ss_workspace::CONFIG_FILE).exists() {
            return Workspace::load(&target_path);
        }
        if target_path.is_absolute() {
            return Err(WorkspaceError::NotAWorkspace);
        }
    }

    // Otherwise, treat it as a workspace name under the configured default root.
    let global = GlobalConfig::load()?;
    let root = global.default_root().ok_or(WorkspaceError::NoDefaultRoot)?;
    Workspace::load(root.join(target))
}

fn resolve_new_target(
    path: Option<Utf8PathBuf>,
    name: Option<String>,
) -> Result<Utf8PathBuf, WorkspaceError> {
    if let Some(p) = path {
        return Ok(expand_tilde_utf8(&p)?);
    }

    if let Some(name) = name {
        let global = GlobalConfig::load()?;
        let root = global.default_root().ok_or(WorkspaceError::NoDefaultRoot)?;
        return Ok(root.join(name));
    }

    // Default: create in current directory.
    current_dir()
}

fn current_dir() -> Result<Utf8PathBuf, WorkspaceError> {
    let cwd = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(cwd).map_err(|p| {
        WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("current directory is not valid UTF-8: {:?}", p),
        ))
    })
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn expand_tilde_utf8(path: &Utf8PathBuf) -> Result<Utf8PathBuf, WorkspaceError> {
    let expanded = expand_tilde(path.as_str());
    Utf8PathBuf::from_path_buf(expanded).map_err(|p| {
        WorkspaceError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("expanded path is not valid UTF-8: {:?}", p),
        ))
    })
}
