# Reports & SOWs

## `sm report` — build a report

Build a report (Markdown to stdout by default):

```sh
sm report acme/web_app/initial
```

Export to HTML or JSON:

```sh
sm report acme/web_app/initial --format html
sm report acme/web_app/initial --format json
```

Export to PDF (requires `--to`):

```sh
sm report acme/web_app/initial --format pdf --to ~/reports/report.pdf
```

Build a report for all findings under a project or client:

```sh
sm report acme/web_app
sm report acme
```

Aggregate across all known workspaces:

```sh
sm report --all --format pdf --to ~/reports/all.pdf
```

### Report assembly

Reports are assembled from findings, scope, document sections, and config data. For PDF output, the Typst engine compiles everything into a professional document.

Config data (client name, contacts, dates) flows from `config.toml` into templates automatically.

### Section control

Include specific sections (comma-separated):

```sh
sm report acme/web_app/initial --format pdf --to report.pdf --sections web/methodology,pricing
```

Exclude specific sections:

```sh
sm report acme/web_app/initial --format pdf --to report.pdf --exclude pricing
```

Sections are discovered by engagement type. A web engagement auto-loads sections from `docs/web/` and `docs/common/`. See [Templates](templates.md) for details on document sections.

### Template selection

Template priority: `--template` flag > `[project.report]` config > `[client.report]` config > built-in default.

```sh
sm report acme/web_app/initial --template custom_report
```

## `sm sow` — build a statement of work

```sh
sm sow acme/web_app/initial
sm sow acme/web_app/initial --format html
sm sow acme/web_app/initial --format pdf --to ~/sows/sow.pdf
```

SOWs are assembled from requirements, scope, document sections, and config data. The same section control and template selection apply:

```sh
sm sow acme/web_app/initial --format pdf --to sow.pdf --sections methodology,pricing,terms
sm sow acme/web_app/initial --format pdf --to sow.pdf --exclude pricing
```

Aggregate across all known workspaces:

```sh
sm sow --all --format pdf --to ~/sows/all.pdf
```

SOWs require an engagement path (client/project/engagement). Template priority: `--template` flag > `[project.sow]` config > `[client.sow]` config > built-in default.