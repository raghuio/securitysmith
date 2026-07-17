# Hierarchy Commands

## `sm new <path>` — create clients, projects, engagements

Create a client, project, or engagement by path depth. Intermediate levels are auto-created if they don't exist:

```sh
sm new acme                    # client
sm new acme/web_app            # project (auto-creates client if missing)
sm new acme/web_app/initial    # engagement (auto-creates client + project if missing)
```

Create the full hierarchy in one command:

```sh
sm new bt/web/eng   # creates client, project, and engagement all at once
```

For engagements, set start and end dates (UTC, YYYY-MM-DD):

```sh
sm new bt/web/eng --start 2026-07-01 --end 2026-07-14
```

Dates are optional. Set them later with `sm edit` or `sm engagement`.

## `sm ls` — list entities

List clients:

```sh
sm ls
```

List projects under a client:

```sh
sm ls acme
```

List engagements under a project:

```sh
sm ls acme/web_app
```

List all content in an engagement (findings, requirements, notes, scope):

```sh
sm ls acme/web_app/initial
```

### Filters

List findings only:

```sh
sm ls acme/web_app/initial --findings
```

Filter findings by severity:

```sh
sm ls acme/web_app/initial --findings --severity high
```

Filter findings by status:

```sh
sm ls acme/web_app/initial --findings --status open
```

List requirements:

```sh
sm ls acme/web_app/initial --requirements
```

List notes:

```sh
sm ls acme/web_app/initial --notes
```

Show scope.md content:

```sh
sm ls acme/web_app/initial --scope
```

List document sections available for this engagement:

```sh
sm ls acme/web_app/initial --sections
```

List custom documents:

```sh
sm ls acme/web_app/initial --documents
```

## `sm show` — show entity details

Show a client's config:

```sh
sm show acme
```

Show a finding by ID:

```sh
sm show ACME-WEB-001
```

Show a template:

```sh
sm show templates/finding
```

## `sm edit` — open config in `$EDITOR`

```sh
sm edit acme
sm edit acme/web_app/initial
sm edit templates/finding
```

This opens the entity's `config.toml` (or template file) in your editor.

## `sm rm` — remove an entity

Remove an entity (moves to OS trash, prompts for confirmation unless `--yes`):

```sh
sm rm acme
sm rm acme --yes
sm rm acme/web_app --yes
sm rm ACME-WEB-001 --yes
```

Without `--yes`, the tool prints a warning and asks `y/N`.

- Items go to the OS trash (Recycle Bin on Windows, Trash on macOS, FreeDesktop Trash on Linux).
- On systems without trash, the tool falls back to permanent deletion with a warning.
- You can remove clients, projects, engagements, findings, requirements, and notes.