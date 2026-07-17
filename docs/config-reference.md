# Config Reference

## Global config (`~/.config/securitysmith/config.toml`)

```toml
[global]
default_workspace = "~/securitysmith"

[[workspace]]
name = "default"
path = "/home/user/securitysmith"

[[workspace]]
name = "2026"
path = "/home/user/securitysmith/2026"
```

The global config tracks all known workspaces and the default workspace. `sm new` registers workspaces here automatically.

## Workspace config.toml

```toml
[workspace]
version = 1
name = "default"
created = "2026-07-08"
```

## Client config.toml

```toml
[client]
status = "active"
priority = "high"
created = "2026-07-08"
updated = "2026-07-08"

[client.id]
prefix = "ACME"
```

The `prefix` is used in finding IDs.

## Project config.toml

```toml
[project]
abbreviation = "WEB"
status = "active"
priority = "high"

[project.id]
sequence = 1
padding = 3
```

The `abbreviation` is used in finding IDs. `sequence` is the per-project counter for finding numbering. `padding` controls zero-padding (3 = `001`).

## Engagement config.toml

```toml
[engagement]
type = "assessment"
status = "in_progress"
start_date = "2026-07-01"
end_date = "2026-07-14"
credentials_ready = false
```

Engagement types: `assessment`, `web`, `api`, `network`, or any custom string. The type controls which document sections are auto-loaded for reports and SOWs.

Engagement statuses: `draft`, `planned`, `in_progress`, `paused`, `completed`, `closed`.

## Report and SOW template config

You can set default report and SOW templates per client or project:

```toml
# In a client config.toml
[client.report]
template = "custom_report"

[client.sow]
template = "custom_sow"
```

```toml
# In a project config.toml
[project.report]
template = "detailed_report"

[project.sow]
template = "fixed_price_sow"
```

Template selection priority: `--template` flag > project config > client config > built-in default.