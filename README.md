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
# Create a workspace
sm new ~/clients/acme-2026
cd ~/clients/acme-2026

# Add a client
sm client add --short acme --display "Acme Corporation"

# List clients
sm client list
```

## Workspace layout

```
~/clients/acme-2026/
├── securitysmith.toml
└── clients/
    └── acme/
        └── client.toml
```

## Configuration

Global config lives at `~/.config/securitysmith/config.toml` and stores:

- `default_workspace_root` — default parent directory for `sm new --name ...`
- known workspaces

## Status

This project is pivoting from a Tauri desktop application to a pure CLI. Only the commands listed under *What is implemented* are functional. The rest of the pentesting lifecycle (projects, engagements, findings, reports, SOWs, exports, backups) is being rebuilt.
