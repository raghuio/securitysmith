# Changelog

All notable changes to SecuritySmith are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/).

## [0.2.0] - 2026-07-16

### Added
- `sm rm` moves deleted items to OS trash (Recycle Bin/FreeDesktop Trash) via `trash` crate. Falls back to permanent deletion with warning when no trash is available.
- Interactive `y/N` confirmation prompt for `sm rm` by default. `--yes` flag skips the prompt for scripts and automation.
- `sm scope <engagement>` now creates `scope.md` from a template on first use. A default `templates/scope.md` is created in the workspace if one doesn't exist. Users edit the template file to change defaults — no recompilation needed.
- `--start` and `--end` optional flags for `sm new` to set engagement dates at creation time (UTC, YYYY-MM-DD).
- `sm new` auto-creates intermediate hierarchy levels: `sm new bt/web/eng` creates client, project, and engagement in one command.
- `{{scope}}` placeholder in report templates — engagement scope content flows into reports.
- Shared `spawn_editor()` function in workspace crate — eliminates duplicated `$EDITOR` spawning code across 4 modules.

### Changed
- `sm rm` exit code 11 repurposed from "Force flag missing" to "Removal declined" (user answered N to prompt).
- `--force` flag replaced with `--yes` flag for `sm rm`.
- Engagement `config.toml` simplified: removed `target` and `notes` fields (violated frontmatter-for-operations-only principle). Only operational fields remain: `type`, `status`, `start_date`, `end_date`.
- Error handling in report/SOW/export commands now propagates read failures instead of silently dropping content.
- `.gitignore` cleaned up: removed stale frontend-era entries, added `core.*` and `*.tmp`.

### Fixed
- `$EDITOR` argument parsing: `$EDITOR` is now split on whitespace before spawning. Previously, the full string (e.g., `emacsclient -c -a emacs`) was passed as a single binary name, causing "No such file or directory" errors.
- Linux seccomp filter no longer kills editor subprocesses. When `$EDITOR` is needed, seccomp is skipped entirely — the editor is a trusted external process.
- Stats module no longer reads client `config.toml` twice — reads once, parses once.

### Security
- New dependency: `trash` v5.2.6 (MIT, actively maintained, no unsafe code).
- `cargo audit`: 0 vulnerabilities.

## [0.1.1] - 2026-07-16

### Added
- `sm list` as a visible alias for `sm ls`.
- Report template config inheritance: `sm report` now resolves templates from `--template` flag > `[project.report]` > `[client.report]` > built-in default.
- SOW template config inheritance: `sm sow` now resolves templates from `--template` flag > `[project.sow]` > `[client.sow]` > built-in default.
- `get_effective_sow_settings()` in entities module for SOW config inheritance lookup.
- `try_workspace_root()` in main module for best-effort workspace root resolution before platform hardening (enables `unveil()` on OpenBSD).
- Tests: invalid requirement status transition returns exit code 8, same-status requirement transition is a no-op, config-based report/SOW template selection.
- Shared workspace walker (`for_each_engagement`, `for_each_md_in_subdir`) in check module to eliminate code duplication.

### Changed
- Invalid requirement status transitions now return exit code 8 (INVALID_STATUS_SEVERITY) instead of exit code 1.
- Same-status requirement transitions (e.g., `open` → `open`) are now allowed as no-ops instead of being rejected.
- Missing config sections (`[project.id]`, `[client.id]`) now return exit code 6 (MISSING_REQUIRED_FIELD) instead of exit code 1.
- Atomic write temp files use consistent `.tmp` extension across all modules (was `.toml.tmp` or `.md.tmp`).
- `sm report` and `sm sow` now use config inheritance chain for template selection per spec.
- `check.rs` refactored to use shared workspace walker, eliminating ~300 lines of duplicated directory traversal code.

### Security
- Upgraded `printpdf` from 0.7.0 to 0.11.1 — fixes high-severity `lopdf` stack overflow vulnerability (RUSTSEC-2026-0187).
- Upgraded `toml` from 0.8 to 1.1.3 — removes `anyhow` unsoundness warning (RUSTSEC-2026-0190) from dependency tree.
- Upgraded `pulldown-cmark` from 0.11 to 0.13.4.
- Upgraded `dirs` from 5 to 6.0.0.
- Upgraded `chrono` to 0.4.45.
- Platform hardening now passes workspace root to `unveil()` on OpenBSD for filesystem restriction.
- `cargo audit`: 0 vulnerabilities (was 1 high + 2 warnings).

## [0.1.0] - 2026-07-10

### Added
- CLI binary `sm` with all commands: new, ls, show, edit, rm, status, check, config, stats, finding, req, scope, note, report, sow
- Workspace initialization with global config tracking
- Path-based hierarchy: clients (depth 1), projects (depth 2), engagements (depth 3)
- Finding management: create with ID generation, status/severity updates, list with filters, export
- Requirement management: create with ID generation, status updates, list, remove
- Scope management: open scope.md in `$EDITOR`
- Note management: create timestamped notes, list, remove
- Template system: built-in defaults, workspace overrides, `--no-template` flag
- Report generation: Markdown, HTML, PDF, JSON output
- SOW generation: Markdown, HTML, PDF, JSON output
- Stats aggregation: per workspace, per client, across all workspaces
- Workspace health checks: duplicate IDs, missing frontmatter fields, invalid values, invalid dates, stale config entries
- Man page generation via clap_mangen (16 per-subcommand pages)
- Strict input validation via clap ValueEnum (severity, status, format)
- Atomic file writes (write to temp, rename) to prevent corruption
- Config versioning for future schema migration
- `NO_COLOR` support
- `./scripts/checks.sh` pre-commit gate (formatting, clippy --all-targets, tests)
- `./scripts/build.sh` for release builds and .deb packaging
- Detailed usage documentation in `docs/usage.md`
- Path traversal protection on all entity, note, and template paths
- Colored output for severity levels and status indicators with `NO_COLOR` and `isatty` detection
- Symlink escape prevention across all modules
- Platform hardening: `pledge`/`unveil` on OpenBSD, `seccomp` on Linux
- Default workspace auto-creation on first use
- Auto-cleanup of stale workspace entries on config load

### Changed
- Finding statuses are `open`, `fixed`, `false_positive`, `not_applicable`, and `risk_accepted`, with free movement between them.

### Security
- Workspace entity, note, and template paths are validated and cannot escape their allowed directories.
- `serde_yaml` replaced with `yaml_serde` (official YAML org fork).

### Removed
- Old desktop crates: core, parsers, report-engine, templates, reports, documents, invoices, analytics, calendar, news, checklists, compliance, email
- Tauri, React/Vite/Mantine frontend
- SQLCipher/SQLite database
- Time tracking
- AI chat assistant
- Email composer
- News feed and RSS aggregation
- Calendar and reminders
- Invoicing and billing
- Analytics dashboard and charts
- Notification center
- Compliance mapping
- Global SQL-backed search index