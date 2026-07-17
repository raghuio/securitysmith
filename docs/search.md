# Search

## `sm search` — search workspace Markdown files

Search across all Markdown files in the current workspace:

```sh
sm search "sql injection"
```

Results are grouped by entity type (findings, requirements, notes, scope, templates, documents). Each result shows the file path, line number, and matching line.

### Filter by entity type

```sh
sm search "sql injection" --type finding
```

Valid types: `finding`, `requirement`, `note`, `scope`, `template`, `document`.

### Limit to a specific client

```sh
sm search "sql injection" --client acme
```

### Notes

- Search is exact substring matching — no fuzzy matching or ranking.
- Binary evidence files are not searched.
- Search covers the current workspace only. Use `sm stats --all` for cross-workspace overview.