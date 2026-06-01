# SecuritySmith Architecture

## Module Boundaries

### `parsers/` — External Format Parsers
- **Rule:** Does not import from `commands/`.
- **Rationale:** Parsers are pure functions (bytes → structured data). They have no knowledge of the database or Tauri commands.
- **Files:** `parsers/mod.rs`, `parsers/nessus.rs`, `parsers/burp.rs`, `parsers/zap.rs`, `parsers/nmap.rs`, `parsers/nuclei.rs`, `parsers/csv_import.rs`

### `report_engine/` — PDF Report Generation
- **Rule:** Does not query the database directly. Takes structs as input.
- **Rationale:** The report engine renders data to PDF. It receives all data from the command layer.
- **Files:** `report_engine/mod.rs`, `report_engine/cover.rs`, `report_engine/summary.rs`, `report_engine/findings_section.rs`, `report_engine/appendix.rs`

### `commands/ai.rs` — AI Orchestration
- **Rule:** Does not directly call other command functions. Uses the same public API.
- **Rationale:** AI tool calls must go through the same validation and audit path as user-initiated actions.
- **Files:** `commands/ai.rs`

### `commands/` — Tauri Command Handlers
- **Rule:** Each module owns one feature area. Cross-feature calls go through the public command API.
- **Files:** `commands/clients.rs`, `commands/engagements.rs`, `commands/findings.rs`, `commands/credentials.rs`, `commands/templates.rs`, `commands/reports.rs`, `commands/documents.rs`, `commands/invoices.rs`, `commands/email.rs`, `commands/calendar.rs`, `commands/news.rs`, `commands/activity_log.rs`, `commands/portability.rs`, `commands/ai.rs`, `commands/auth.rs`, `commands/settings.rs`

### `db.rs` — Database Layer
- **Rule:** All SQL lives here or in migrations. Commands never write raw SQL.
- **Rationale:** Centralized schema management, consistent connection handling.

### `crypto.rs` — Cryptography
- **Rule:** Only module that touches Argon2id, salts, and key derivation.
- **Rationale:** Cryptographic operations must be isolated and auditable.

## Dependency Flow

```
frontend (React) → Tauri invoke → commands/ → db.rs → SQLCipher vault
                                    ↓
                              crypto.rs (key derivation)
                              parsers/ (external formats)
                              report_engine/ (PDF generation)
```

## State Management

- **Rust:** Single `Arc<Mutex<Option<Connection>>>` in Tauri managed state.
- **Frontend:** React local state + Mantine form hooks. No global state library.

## Data Flow

1. User action in React component
2. Typed API wrapper calls `invoke()`
3. Tauri routes to command handler in `commands/`
4. Command acquires DB connection from state
5. Command runs parameterized SQL via `db.rs`
6. Command writes to `audit_log`
7. Command returns `Result<T, String>`
8. Frontend receives typed result and updates UI

## Security Boundaries

- **Vault encryption:** SQLCipher AES-256, key derived via Argon2id.
- **No plaintext secrets:** Passwords, SMTP passwords, recovery phrases never logged.
- **No external telemetry:** All network calls are user-initiated (Ollama, SMTP, RSS).
- **Audit trail:** Every mutation recorded in `audit_log` with old/new JSON snapshots.
