# SecuritySmith Documentation

SecuritySmith (`sm`) is a privacy-first CLI tool for security consultants and penetration testers. All data lives in the filesystem as Markdown and TOML files. No database, no UI, no network calls, no telemetry.

## Quick start

```sh
# Install
cargo install --path crates/cli

# Create a workspace
sm new

# Create a client, project, and engagement in one command
sm new acme/web_app/initial

# Add a finding
sm finding acme/web_app/initial --title "Stored XSS"

# List findings
sm ls acme/web_app/initial --findings

# Update finding status
sm finding ACME-WEB-001 --status fixed

# View stats
sm stats

# Check workspace health
sm check
```

## Guides

| Guide | Covers |
|-------|--------|
| [Getting Started](getting-started.md) | Installation, core concepts, hierarchy, data formats, naming |
| [Workspace](workspace.md) | `sm new`, `sm status`, `sm config`, `sm check`, `sm stats` |
| [Hierarchy](hierarchy.md) | `sm ls`, `sm show`, `sm edit`, `sm rm` |
| [Engagement Management](engagement.md) | `sm engagement` — status, dates, retest, engagement overview |
| [Findings](findings.md) | `sm finding` — create, update, export, import, retest/remediation |
| [Requirements](requirements.md) | `sm req` — create, update, export |
| [Scope & Notes](scope-notes.md) | `sm scope`, `sm note` |
| [Evidence](evidence.md) | `sm evidence` — add, list, show evidence files |
| [Credentials](credentials.md) | `sm credential` — encrypted credential store |
| [Checklists](checklists.md) | `sm checklist` — methodology tracking and coverage |
| [Search](search.md) | `sm search` — workspace-wide search |
| [Reports & SOWs](reports.md) | `sm report`, `sm sow` — assembly, formats, sections |
| [Custom Documents](documents.md) | `sm document` — RoE, NDA, proposal, custom |
| [Templates](templates.md) | Markdown and Typst templates, document sections |
| [Backup & Version Control](backup.md) | `tar` backup, `git` tracking |
| [Config Reference](config-reference.md) | `config.toml` formats for all entity levels |

## Command reference

| Command | Alias | Description |
|---------|-------|-------------|
| `sm new` | `n` | Create workspace, client, project, engagement, or template |
| `sm ls` | `l`, `list` | List entities |
| `sm show` | `s` | Show entity details |
| `sm edit` | `e` | Open config in `$EDITOR` |
| `sm rm` | `r` | Remove entity (moves to trash, prompts unless `--yes`) |
| `sm status` | `st` | Show workspace info, or engagement overview for a client/all workspaces |
| `sm check` | `c` | Check workspace health |
| `sm config` | `cfg` | Show or set global config |
| `sm stats` | — | Show statistics |
| `sm engagement` | `eng` | Manage engagement status and fields |
| `sm checklist` | `cl` | Manage methodology checklists |
| `sm search` | — | Search workspace Markdown files |
| `sm credential` | `cred` | Manage credentials (encrypted store) |
| `sm evidence` | `ev` | Manage evidence files |
| `sm finding` | `f` | Manage findings |
| `sm req` | `requirement` | Manage requirements |
| `sm scope` | — | Edit or export scope |
| `sm note` | — | Create notes or export notes |
| `sm report` | — | Build reports |
| `sm sow` | — | Build statements of work |
| `sm document` | `doc` | Manage custom documents |

## Global options

```sh
# Use a specific workspace by name or path
sm -w 2026 ls
sm -w /path/to/workspace ls
```

## Help

```sh
sm --help          # top-level help
sm finding --help  # help for a specific command
man sm             # man pages (generate with ./scripts/build.sh --man)
```