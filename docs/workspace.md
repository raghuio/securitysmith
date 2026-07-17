# Workspace Commands

## `sm new` — create a workspace

Create a workspace in the current directory:

```sh
sm new
```

Create a workspace at a specific path:

```sh
sm new ~/securitysmith/2026
sm new /path/to/workspace
```

The workspace is registered in global config automatically.

## `sm status` — engagement overview

`sm status` shows engagement status and scheduling. See [Engagement Management](engagement.md) for full details.

```sh
sm status                    # active engagements across all workspaces
sm status -w 2026           # active engagements in one workspace
sm status acme              # active engagements for one client
sm status acme --archived   # completed/closed only
sm status acme --all        # all statuses
sm status --archived        # archived across all workspaces
sm status --all             # everything across all workspaces
```

If no engagements match the filter, prints "No active engagements found." (or "No archived engagements found.")

## `sm config` — global config

Show global config:

```sh
sm config
```

Set default workspace:

```sh
sm config set default_workspace ~/securitysmith/2026
```

The global config lives at `~/.config/securitysmith/config.toml`. See [Config Reference](config-reference.md).

## `sm check` — workspace health

```sh
sm check
```

Checks for:
- Stale workspace entries in global config
- Duplicate finding IDs
- Missing frontmatter fields
- Invalid status or severity values
- Invalid dates
- Evidence files with mismatched hashes
- Secrets in evidence files (text files only)
- Credential store integrity

Remove stale workspace entries:

```sh
sm check --fix
```

## `sm stats` — statistics

Stats for the current workspace:

```sh
sm stats
```

Stats for a single client:

```sh
sm stats acme
```

Stats across all known workspaces:

```sh
sm stats --all
```

Output includes client/project/engagement counts, findings by severity, engagements by status, and open findings per project.