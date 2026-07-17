# Engagement Management

## `sm engagement` — manage engagement status and fields

### View current config

```sh
sm engagement acme/web_app/initial
```

Prints the engagement's `config.toml`.

### Update status

```sh
sm engagement acme/web_app/initial --status in_progress
```

Valid statuses: `draft`, `planned`, `in_progress`, `paused`, `completed`, `closed`.

### Update dates

```sh
sm engagement acme/web_app/initial --start-date 2026-07-01
sm engagement acme/web_app/initial --end-date 2026-07-14
```

Dates are in YYYY-MM-DD format (UTC).

### Toggle credential readiness

```sh
sm engagement acme/web_app/initial --credentials-ready
```

Toggles a boolean flag in the engagement config. Use this to mark that test credentials have been received and verified.

### Create a retest engagement

```sh
sm engagement acme/web_app/retest_01 --retest --from acme/web_app/initial
```

Creates a new engagement and copies findings from the original engagement that are in `fixed` or `risk_accepted` status. Copied findings get `retest_result = not_tested` and new IDs for the retest context.

The original engagement is linked via an `original_engagement` field in the retest engagement's `config.toml`.

## `sm status` — engagement overview

`sm status` is a dashboard that shows engagement status and scheduling across your workspaces.

### All workspaces (default)

```sh
sm status
```

Shows all **active** engagements (draft, planned, in_progress, paused) across every known workspace, grouped by workspace → client → project:

```
Workspace: default (/home/user/securitysmith)

  acme/
    web/
      initial         in_progress   2026-07-01 → 2026-07-14
    api/
      v1_assess       planned       2026-08-01 → 2026-08-07

Workspace: 2026 (/home/user/securitysmith/2026)

  bt/
    web/
      initial         draft         —
```

### One workspace

```sh
sm status -w 2026
```

Shows active engagements in the `2026` workspace only.

### One client

```sh
sm status acme
```

Shows active engagements for client `acme` in the current (or `-w` specified) workspace, one line per engagement:

```
Client: acme

web/initial              in_progress    2026-07-01 → 2026-07-14
web/retest_01            planned       2026-08-01 → 2026-08-07
```

### Archived engagements

```sh
sm status acme --archived
sm status --archived
```

Shows only completed and closed engagements (for a client, or across all workspaces).

### All engagements

```sh
sm status acme --all
sm status --all
```

Shows all engagements regardless of status. If both `--archived` and `--all` are set, `--all` wins.

### Status reference

| Status | Group | Meaning |
|--------|-------|---------|
| `draft` | active | Engagement created, not yet planned |
| `planned` | active | Scheduled, about to start |
| `in_progress` | active | Work in progress |
| `paused` | active | Temporarily paused |
| `completed` | archived | Work finished |
| `closed` | archived | Closed out, no further action |