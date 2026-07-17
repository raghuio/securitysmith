# Getting Started

## Installation

```sh
# From source
cargo install --path crates/cli

# Build a .deb package
./scripts/build.sh --deb
sudo dpkg -i dist/*.deb
```

The binary is installed as `sm`.

## Core concepts

### Workspace

A workspace is your top-level container. It's a directory with a `config.toml` containing a `[workspace]` section. Create one with `sm new`.

Default location: `~/securitysmith/`. Change with `sm config set default_workspace <path>`.

### Hierarchy

```
workspace/
  config.toml              # [workspace] section
  templates/               # workspace-level template overrides
  docs/                    # document sections (reusable content)
  acme/                    # client (depth 1)
    config.toml             # [client] section
    web_app/                # project (depth 2)
      config.toml           # [project] section
      initial/              # engagement (depth 3)
        config.toml         # [engagement] section
        scope.md            # scope (pure Markdown)
        findings/
          acme_web_001_stored_xss.md
        requirements/
          req_001_test_auth.md
        notes/
          2026_07_02_recon.md
        evidence/
          screenshot_01.png
        checklist.toml       # methodology tracking
```

Depth tells you the entity type:
- **Depth 1** = client
- **Depth 2** = project
- **Depth 3** = engagement

No wrapper directories (`clients/`, `projects/`, `engagements/`). The directory name is the entity name.

### Data formats

| Type | Format | Notes |
|------|--------|-------|
| Settings | TOML | `config.toml` in every entity directory |
| Findings | Markdown + YAML frontmatter | Fields: `id`, `status`, `severity`, `created`, `updated` |
| Requirements | Markdown + YAML frontmatter | Fields: `id`, `status`, `created`, `updated` |
| Notes | Markdown + minimal frontmatter | Fields: `id`, `created`, `updated` |
| Scope | Pure Markdown | No frontmatter |
| Templates | Pure Markdown | No frontmatter |
| Credentials | Encrypted file (`.credentials.enc`) | ChaCha20-Poly1305 + Argon2id |

### Naming

All entity names are `snake_case` (lowercase letters, digits, underscores). The name is the directory name.

### Finding IDs

Format: `CLIENT_PREFIX-PROJECT_ABBR-SEQUENCE`

Example: `ACME-WEB-001`

- `ACME` comes from the client's `[client.id]` prefix
- `WEB` comes from the project's abbreviation
- `001` is a per-project sequence counter

### Template priority

1. Workspace templates (`templates/` directory)
2. Built-in defaults
3. None (frontmatter only)

Use `--no-template` to skip templates entirely.

## First steps

1. [Create a workspace](workspace.md)
2. [Set up your hierarchy](hierarchy.md)
3. [Add findings](findings.md)
4. [Generate a report](reports.md)