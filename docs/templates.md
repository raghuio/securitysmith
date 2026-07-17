# Templates

## Markdown templates

Templates are the starting point for new findings, requirements, reports, SOWs, and scope files. Built-in defaults ship with the binary. Override them by placing your own files in the workspace `templates/` directory.

### List templates

```sh
sm ls templates
```

Shows template name and source (workspace or built-in).

### Create a workspace template

```sh
sm new templates/finding
sm new templates/report
sm new templates/sow
sm new templates/requirement
```

This copies the built-in default as a starting point and opens it in `$EDITOR`.

### Show a template

```sh
sm show templates/finding
```

### Edit a template

```sh
sm edit templates/finding
```

### Remove a workspace template

```sh
sm rm templates/finding --yes
```

You cannot remove built-in templates. Removing a workspace template falls back to the built-in default.

## Typst templates (PDF rendering)

PDF export uses Typst templates (`.typ` files). Built-in skeleton templates ship with the binary — they work out of the box. Override them by placing your own `.typ` files in `templates/`:

```
templates/
  report.typ       # Report PDF template
  sow.typ          # SOW PDF template
  proposal.typ     # Proposal PDF template
  finding.typ      # Single finding PDF template
  requirement.typ  # Single requirement PDF template
  scope.typ        # Scope PDF template
  note.typ         # Notes PDF template
  roe.typ          # Rules of Engagement template
  nda.typ          # NDA template
  custom.typ       # Custom document template
```

Templates receive data as named arguments: `findings`, `requirements`, `scope`, `sections`, `metadata`, `notes`, and `doc`.

## Document sections

Document sections are reusable Markdown files in `docs/` directories. They are the building blocks for reports and SOWs. Organized by hierarchy level and engagement type:

```
docs/
  common/           # Shared across all engagement types
    terms.md
    document_control.md
  web/              # Web pentest sections
    methodology.md
    pricing.md
  api/              # API pentest sections
    methodology.md
    pricing.md
  network/          # Network pentest sections
    methodology.md
    pricing.md
```

### Inheritance

Sections are discovered from engagement → project → client → workspace. First match wins. Write methodology once at the workspace level, reuse for every client.

### Type filtering

When generating a report for a web engagement, the tool auto-loads sections from `docs/web/` and `docs/common/`. API sections are ignored unless explicitly requested with `--sections`.

### List available sections

```sh
sm ls acme/web_app/initial --sections
```

### Override sections in reports and SOWs

```sh
# Include specific sections
sm report acme/web_app/initial --sections web/methodology,pricing

# Exclude specific sections
sm report acme/web_app/initial --exclude pricing
```

See [Reports & SOWs](reports.md) for full assembly details.