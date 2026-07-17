# Findings

## `sm finding` — manage findings

### Create a finding

```sh
sm finding acme/web_app/initial --title "Stored XSS"
```

This creates the finding file and opens it in `$EDITOR`. Skip the template:

```sh
sm finding acme/web_app/initial --title "Stored XSS" --no-template
```

### Show a finding

```sh
sm finding ACME-WEB-001
```

### Update finding status

```sh
sm finding ACME-WEB-001 --status fixed
```

Valid statuses: `open`, `fixed`, `false_positive`, `not_applicable`, `risk_accepted`. You can change freely between any of them.

### Update finding severity

```sh
sm finding ACME-WEB-001 --severity critical
```

Valid severities: `critical`, `high`, `medium`, `low`, `informational`.

### Export findings

Export a single finding:

```sh
sm finding ACME-WEB-001 --export html
sm finding ACME-WEB-001 --export pdf --to ~/reports/finding.pdf
sm finding ACME-WEB-001 --export json
sm finding ACME-WEB-001 --export markdown
```

Export all findings in an engagement:

```sh
sm finding acme/web_app/initial --export html
```

Export all findings under a project or client:

```sh
sm finding acme/web_app --export json
sm finding acme --export markdown
```

Formats: `markdown`, `html`, `pdf`, `json`. PDF requires `--to <path>`.

## Import findings from scanner output

```sh
sm finding acme/web_app/initial --import ~/scans/results.nessus --import-format nessus
```

Import from CSV:

```sh
sm finding acme/web_app/initial --import ~/scans/results.csv \
  --import-format csv \
  --title-column 0 \
  --severity-column 3
```

Supported formats:
- **Nessus** — XML export
- **CSV** — generic, requires `--title-column` and `--severity-column` (0-based index)

Each imported finding becomes a Markdown file with YAML frontmatter. Severity is mapped to SecuritySmith's 5-level scale. Duplicate detection skips findings with the same title already in the engagement.

The import prints a summary: how many parsed, how many created, how many duplicates skipped.

## Retest and remediation tracking

### Set retest result

```sh
sm finding ACME-WEB-001 --retest-result pass
```

Valid values: `not_tested`, `pass`, `fail`, `partial`.

### Set client response

```sh
sm finding ACME-WEB-001 --client-response fixed
```

Valid values: `acknowledged`, `in_progress`, `fixed`, `accepted_risk`, `disputed`, `deferred`, `no_response`.

### Set fix deadline

```sh
sm finding ACME-WEB-001 --fix-deadline 2026-08-15
```

Auto-calculate from severity:

```sh
sm finding ACME-WEB-001 --fix-deadline auto
```

Auto-calculation uses: critical=30 days, high=60 days, medium=90 days, low=180 days, informational=none.