# SecuritySmith

SecuritySmith is a command-line tool for security consultants and penetration testers to manage client work using plain-text files.

> ⚠️ Work in progress. Core features are being rebuilt as a CLI and the codebase is not yet ready for production use.

## What it is today

A single Rust binary named `sm` that creates and manages a local workspace of files:

- **TOML** for configuration and structured metadata
- **Markdown with YAML frontmatter** for notes and content
- **Filesystem directories** for the client → project → engagement hierarchy

Everything is stored locally and is git-friendly.

## What is implemented

- `sm new [path]` — create a workspace
- `sm here` — show the current workspace
- `sm config` / `sm config set` — manage global configuration
- `sm client add|list|rm|rename|move` — manage clients

## Build

```bash
cargo build --release
```

The `sm` binary is produced in `target/release/sm`.

## Quick start

```bash
# Set the default workspace root (optional)
sm config set default_workspace_root ~/securitysmith

# Create a workspace under the default root
sm new --name acme-2026
cd ~/securitysmith/acme-2026

# Add a client
sm client add --short acme --display "Acme Corporation"

# List clients
sm client list
```

## Workspace layout

```
~/securitysmith/
└── acme-2026/
    ├── securitysmith.toml
    └── clients/
        └── acme/
            └── client.toml
```

## Configuration

Global config lives at the OS default config location under `securitysmith/config.toml`:

- Linux: `~/.config/securitysmith/config.toml`
- macOS: `~/Library/Application Support/securitysmith/config.toml`
- Windows: `%APPDATA%\securitysmith\config.toml`

It stores:

- `default_workspace_root` — default parent directory for `sm new --name ...` (defaults to `~/securitysmith` if not set)
- known workspaces

## Status

This project is pivoting from a Tauri desktop application to a pure CLI. Only the commands listed under *What is implemented* are functional. The rest of the pentesting lifecycle (projects, engagements, findings, reports, SOWs, exports, backups) is being rebuilt.
