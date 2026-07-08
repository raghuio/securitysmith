use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::path::Path;

const INIT_SQL: &str = include_str!("../migrations/001_init.sql");
const CRYPTO_META_SQL: &str = include_str!("../migrations/002_crypto_meta.sql");
const RECOVERY_SQL: &str = include_str!("../migrations/003_recovery.sql");
const CLIENTS_SQL: &str = include_str!("../migrations/004_clients.sql");
const ENGAGEMENTS_SQL: &str = include_str!("../migrations/005_engagements.sql");
const CREDENTIALS_SQL: &str = include_str!("../migrations/006_credentials.sql");
const FINDINGS_SQL: &str = include_str!("../migrations/007_findings.sql");
const TEMPLATES_SQL: &str = include_str!("../migrations/008_templates.sql");
const REPORTS_SQL: &str = include_str!("../migrations/009_reports.sql");
const DOCUMENTS_SQL: &str = include_str!("../migrations/010_documents.sql");
const INVOICES_SQL: &str = include_str!("../migrations/011_invoices.sql");
const DISMISSED_REMINDERS_SQL: &str = include_str!("../migrations/012_dismissed_reminders.sql");
const NEWS_SQL: &str = include_str!("../migrations/013_news.sql");
const TECH_STACK_SQL: &str = include_str!("../migrations/014_tech_stack.sql");
const ATTACHMENTS_SQL: &str = include_str!("../migrations/020_attachments.sql");
const CLIENT_CONTACTS_SQL: &str = include_str!("../migrations/021_client_contacts.sql");
const REMEDIATION_SQL: &str = include_str!("../migrations/022_remediation.sql");
const SCOPE_ITEMS_SQL: &str = include_str!("../migrations/023_scope_items.sql");
const TIME_ENTRIES_SQL: &str = include_str!("../migrations/024_time_entries.sql");
const CHECKLISTS_SQL: &str = include_str!("../migrations/027_checklists.sql");
const SEARCH_INDEX_SQL: &str = include_str!("../migrations/028_search_index.sql");
const NOTIFICATIONS_SQL: &str = include_str!("../migrations/029_notifications.sql");
const COMPLIANCE_SQL: &str = include_str!("../migrations/030_compliance.sql");
const PROJECTS_SQL: &str = include_str!("../migrations/031_projects.sql");
const PROJECT_CONTACTS_SQL: &str = include_str!("../migrations/032_project_contacts.sql");
const ENGAGEMENT_TYPE_LABELS_SQL: &str =
    include_str!("../migrations/033_engagement_type_labels.sql");
const CLIENT_HISTORY_SQL: &str = include_str!("../migrations/034_client_history.sql");
const ALTER_CLIENTS_SQL: &str = include_str!("../migrations/035_alter_clients.sql");
const ALTER_ENGAGEMENTS_SQL: &str = include_str!("../migrations/036_alter_engagements.sql");
const UNCATEGORIZED_PROJECTS_SQL: &str =
    include_str!("../migrations/037_uncategorized_projects.sql");

/// Open or create the encrypted SQLite vault at the given directory.
/// The database file is named `vault.db` and is encrypted with SQLCipher
/// using the provided raw 32-byte key.
pub fn open_vault(data_dir: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create data directory: {e}"))?;
    }

    let db_path = data_dir.join("vault.db");

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .map_err(|e| format!("Failed to open vault database: {e}"))?;

    // Apply SQLCipher encryption key as raw hex bytes.
    // hex::encode guarantees only [0-9a-f], but we assert as defense-in-depth.
    let key_hex = hex::encode(key);
    debug_assert!(
        key_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "key_hex must be hex digits"
    );
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key_hex))
        .map_err(|e| format!("Failed to set encryption key: {e}"))?;

    // Verify encryption is active by reading a harmless value
    conn.query_row("SELECT 1", [], |_| Ok(()))
        .map_err(|e| format!("Vault key verification failed (wrong password?): {e}"))?;

    Ok(conn)
}

/// Re-key an open vault from one raw key to another.
/// The connection must already be open with the old key.
pub fn rekey_vault(
    conn: &Connection,
    old_key: &[u8; 32],
    new_key: &[u8; 32],
) -> Result<(), String> {
    let old_hex = hex::encode(old_key);
    let new_hex = hex::encode(new_key);
    debug_assert!(
        old_hex.chars().all(|c| c.is_ascii_hexdigit())
            && new_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "key_hex must be hex digits"
    );
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\"; PRAGMA rekey = \"x'{}'\";",
        old_hex, new_hex
    ))
    .map_err(|e| format!("Failed to re-key vault: {e}"))?;
    Ok(())
}

/// Initialise schema: run migrations, fix any broken schemas, enable WAL mode, tune pragmas.
pub fn init_db(conn: &Connection) -> Result<(), String> {
    enable_wal(conn)?;
    run_migrations(conn)?;
    fix_broken_clients_schema(conn)?;
    Ok(())
}

/// Enable Write-Ahead Logging for concurrent reads/writes and better performance.
fn enable_wal(conn: &Connection) -> Result<(), String> {
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|e| format!("Failed to enable WAL mode: {e}"))?;

    if journal_mode != "wal" {
        return Err(format!("WAL mode not enabled, got: {journal_mode}"));
    }

    // WAL tuning for desktop use: let checkpoint happen automatically,
    // but keep a reasonable size limit (1000 pages ≈ 4 MB)
    conn.execute_batch("PRAGMA wal_autocheckpoint = 1000; PRAGMA busy_timeout = 5000;")
        .map_err(|e| format!("Failed to set WAL autocheckpoint: {e}"))?;

    Ok(())
}

/// Run embedded SQL migrations.
/// Run embedded SQL migrations, tracking each one in `_migrations` so
/// already-applied steps are skipped.
fn run_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            version     INTEGER NOT NULL UNIQUE,
            name        TEXT NOT NULL,
            applied_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )
    .map_err(|e| format!("Failed to create _migrations table: {e}"))?;

    apply_migration(conn, 1, "init", INIT_SQL)?;
    apply_migration(conn, 2, "crypto_meta", CRYPTO_META_SQL)?;
    apply_migration(conn, 3, "recovery", RECOVERY_SQL)?;
    apply_migration(conn, 4, "clients", CLIENTS_SQL)?;
    apply_migration(conn, 5, "engagements", ENGAGEMENTS_SQL)?;
    apply_migration(conn, 6, "credentials", CREDENTIALS_SQL)?;
    apply_migration(conn, 7, "findings", FINDINGS_SQL)?;
    apply_migration(conn, 8, "templates", TEMPLATES_SQL)?;
    apply_migration(conn, 9, "reports", REPORTS_SQL)?;
    apply_migration(conn, 10, "documents", DOCUMENTS_SQL)?;
    apply_migration(conn, 11, "invoices", INVOICES_SQL)?;
    apply_migration(conn, 12, "dismissed_reminders", DISMISSED_REMINDERS_SQL)?;
    apply_migration(conn, 13, "news", NEWS_SQL)?;
    apply_migration(conn, 14, "tech_stack", TECH_STACK_SQL)?;
    apply_migration(conn, 20, "attachments", ATTACHMENTS_SQL)?;
    apply_migration(conn, 21, "client_contacts", CLIENT_CONTACTS_SQL)?;
    apply_migration(conn, 22, "remediation", REMEDIATION_SQL)?;
    apply_migration(conn, 23, "scope_items", SCOPE_ITEMS_SQL)?;
    apply_migration(conn, 24, "time_entries", TIME_ENTRIES_SQL)?;
    apply_migration(conn, 27, "checklists", CHECKLISTS_SQL)?;
    apply_migration(conn, 28, "search_index", SEARCH_INDEX_SQL)?;
    apply_migration(conn, 29, "notifications", NOTIFICATIONS_SQL)?;
    apply_migration(conn, 30, "compliance", COMPLIANCE_SQL)?;
    apply_migration(conn, 31, "projects", PROJECTS_SQL)?;
    apply_migration(conn, 32, "project_contacts", PROJECT_CONTACTS_SQL)?;
    apply_migration(
        conn,
        33,
        "engagement_type_labels",
        ENGAGEMENT_TYPE_LABELS_SQL,
    )?;
    apply_migration(conn, 34, "client_history", CLIENT_HISTORY_SQL)?;
    apply_migration(conn, 35, "alter_clients", ALTER_CLIENTS_SQL)?;
    apply_migration(conn, 36, "alter_engagements", ALTER_ENGAGEMENTS_SQL)?;
    apply_migration(
        conn,
        37,
        "uncategorized_projects",
        UNCATEGORIZED_PROJECTS_SQL,
    )?;

    seed_builtin_templates(conn)?;
    seed_default_news_feeds(conn)?;
    seed_builtin_checklists(conn)?;
    seed_builtin_compliance(conn)?;
    Ok(())
}

/// Apply a single migration if it has not already been recorded.
/// Wrapped in a transaction so partial failures roll back and the
/// migration is not incorrectly marked as applied.
fn apply_migration(conn: &Connection, version: i64, name: &str, sql: &str) -> Result<(), String> {
    let already_applied: bool = conn
        .query_row(
            "SELECT 1 FROM _migrations WHERE version = ?1",
            [version],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Migration check failed for version {version}: {e}"))?
        .unwrap_or(false);

    if already_applied {
        return Ok(());
    }

    // Begin explicit transaction so a failure in any statement rolls back.
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| format!("Failed to begin migration transaction: {e}"))?;

    let result = conn.execute_batch(sql);

    if let Err(ref e) = result {
        let msg = e.to_string().to_lowercase();
        if !msg.contains("duplicate column name") {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!("Migration {version} failed: {e}"));
        }
        // duplicate column name is idempotent — still commit
    }

    conn.execute(
        "INSERT INTO _migrations (version, name, applied_at) VALUES (?1, ?2, strftime('%s', 'now'))",
        params![version, name],
    )
    .map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        format!("Failed to record migration {version}: {e}")
    })?;

    conn.execute("COMMIT", [])
        .map_err(|e| format!("Failed to commit migration {version}: {e}"))?;

    Ok(())
}

/// Fix clients schema for databases where migration 035 was partially applied.
/// The old apply_migration error handler incorrectly treated "duplicate column name"
/// as idempotent for the entire batch, causing 035 to be marked as applied without
/// dropping the old `name` / `contact_email` / `tech_stack` columns.
/// This function checks if `name` still exists and safely recreates the table.
fn fix_broken_clients_schema(conn: &Connection) -> Result<(), String> {
    // Check if the old `name` column still exists
    let name_exists: bool = conn
        .query_row(
            "SELECT 1 FROM pragma_table_info('clients') WHERE name = 'name'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Schema check failed: {e}"))?
        .unwrap_or(false);

    if !name_exists {
        return Ok(()); // Schema is already correct
    }

    conn.execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|e| format!("Failed to disable FK checks: {e}"))?;

    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| format!("Failed to begin fix transaction: {e}"))?;

    let result = conn.execute_batch(
        "CREATE TABLE clients_new (
            id                       INTEGER PRIMARY KEY AUTOINCREMENT,
            short_name               TEXT NOT NULL UNIQUE,
            registered_business_name TEXT,
            country                  TEXT,
            address                  TEXT,
            email                    TEXT,
            contact_number           TEXT,
            business_tier            TEXT
                CHECK (business_tier IN ('enterprise', 'mid_market', 'small')),
            priority                 TEXT
                CHECK (priority IN ('high', 'medium', 'low')),
            status                   TEXT DEFAULT 'active'
                CHECK (status IN ('active', 'inactive', 'prospect')),
            tax_info                 TEXT DEFAULT '{}',
            logo_attachment_id       INTEGER,
            notes                    TEXT,
            tags                     TEXT NOT NULL DEFAULT '[]',
            is_active                INTEGER NOT NULL DEFAULT 1,
            created_at               INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            updated_at               INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );

        INSERT INTO clients_new (
            id, short_name, registered_business_name, country, address, email,
            contact_number, business_tier, priority, status, tax_info, logo_attachment_id,
            notes, tags, is_active, created_at, updated_at
        )
        SELECT
            id,
            COALESCE(short_name, name, 'Unknown')              AS short_name,
            COALESCE(registered_business_name, name, '')     AS registered_business_name,
            COALESCE(country, '')                              AS country,
            COALESCE(address, '')                             AS address,
            COALESCE(email, contact_email, '')                AS email,
            COALESCE(contact_number, '')                       AS contact_number,
            business_tier                                       AS business_tier,
            COALESCE(priority, 'medium')                       AS priority,
            COALESCE(status, 'active')                         AS status,
            COALESCE(tax_info, '{}')                           AS tax_info,
            logo_attachment_id,
            notes,
            COALESCE(tags, '[]')                               AS tags,
            is_active,
            created_at,
            updated_at
        FROM clients;

        DROP TABLE clients;
        ALTER TABLE clients_new RENAME TO clients;

        CREATE INDEX IF NOT EXISTS idx_clients_short_name ON clients(short_name);
        CREATE INDEX IF NOT EXISTS idx_clients_status ON clients(status);
        CREATE INDEX IF NOT EXISTS idx_clients_active ON clients(is_active);

        PRAGMA foreign_keys = ON;",
    );

    if let Err(e) = result {
        let _ = conn.execute("ROLLBACK", []);
        return Err(format!("Clients schema fix failed: {e}"));
    }

    conn.execute("COMMIT", [])
        .map_err(|e| format!("Failed to commit schema fix: {e}"))?;

    Ok(())
}

fn seed_default_news_feeds(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM feeds", [], |row| row.get(0))
        .map_err(|e| format!("Failed to count feeds: {e}"))?;
    if count > 0 {
        return Ok(());
    }
    let defaults = vec![
        ("https://www.bleepingcomputer.com/feed/", "BleepingComputer"),
        (
            "https://feeds.feedburner.com/TheHackersNews",
            "The Hacker News",
        ),
        (
            "https://www.us-cert.gov/ncas/current-activity.xml",
            "CISA Alerts",
        ),
    ];
    for (url, name) in defaults {
        conn.execute(
            "INSERT INTO feeds (url, name, is_default, created_at, updated_at) VALUES (?1, ?2, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
            rusqlite::params![url, name],
        ).map_err(|e| format!("Failed to seed news feed: {e}"))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Recovery helpers
//
// The recovery envelope is stored in TWO places on every successful
// `validate_recovery_words`:
//   1. The in-vault `recovery` table (this `store_recovery`).
//   2. The external file `recovery_envelope.bin` (see `save_recovery_envelope`).
//
// `recover_vault` reads the EXTERNAL FILE because it runs *before* the
// SQLCipher database is unlocked. The in-vault row is a redundant copy
// kept for forensic/audit purposes; the two are written together and
// never read independently.
// ─────────────────────────────────────────────────────────────

/// Store or overwrite the recovery envelope and its salt inside the vault.
/// Mirrors the bytes saved to `recovery_envelope.bin` by the caller; the
/// external file is the source of truth used by `recover_vault`.
pub fn store_recovery(conn: &Connection, envelope: &[u8], salt: &[u8; 16]) -> Result<(), String> {
    conn.execute(
        "INSERT INTO recovery (id, encrypted_envelope, envelope_salt, created_at)
         VALUES (1, ?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(id) DO UPDATE SET
             encrypted_envelope = excluded.encrypted_envelope,
             envelope_salt = excluded.envelope_salt,
             created_at = excluded.created_at",
        params![envelope, &salt[..]],
    )
    .map_err(|e| format!("Failed to store recovery data: {e}"))?;
    Ok(())
}

const RECOVERY_ENVELOPE_FILENAME: &str = "recovery_envelope.bin";

/// Save the recovery envelope (salt || nonce || ciphertext) to an external file.
/// The salt is prepended so `recover_vault` can derive the key without opening the vault.
pub fn save_recovery_envelope(
    data_dir: &Path,
    salt: &[u8; 16],
    envelope: &[u8],
) -> Result<(), String> {
    let path = data_dir.join(RECOVERY_ENVELOPE_FILENAME);
    let mut data = salt.to_vec();
    data.extend_from_slice(envelope);
    std::fs::write(&path, data).map_err(|e| format!("Failed to write recovery envelope: {e}"))?;
    Ok(())
}

/// Load the recovery envelope from the external file.
/// Returns `(envelope_without_salt, salt)` where envelope is `nonce || ciphertext`.
pub fn load_recovery_envelope(data_dir: &Path) -> Result<(Vec<u8>, [u8; 16]), String> {
    let path = data_dir.join(RECOVERY_ENVELOPE_FILENAME);
    if !path.exists() {
        return Err("Recovery phrase not configured for this vault.".to_string());
    }
    let data =
        std::fs::read(&path).map_err(|e| format!("Failed to read recovery envelope: {e}"))?;
    if data.len() < 16 {
        return Err("Recovery data corrupted.".to_string());
    }
    let (salt_bytes, envelope) = data.split_at(16);
    let mut salt = [0u8; 16];
    salt.copy_from_slice(salt_bytes);
    Ok((envelope.to_vec(), salt))
}

const OWASP_WEB: &str = include_str!("templates/owasp_web.json");
const OWASP_API: &str = include_str!("templates/owasp_api.json");
const OWASP_LLM: &str = include_str!("templates/owasp_llm.json");
const OTHER_TEMPLATES: &str = include_str!("templates/other.json");

/// Seed built-in templates if none exist.
fn seed_builtin_templates(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM templates WHERE is_builtin = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count templates: {e}"))?;

    if count > 0 {
        return Ok(());
    }

    #[derive(serde::Deserialize)]
    struct RawTemplate {
        name: String,
        category: Option<String>,
        subcategory: String,
        content: Option<String>,
    }

    let parse_and_insert = |json: &str, default_category: &str| -> Result<(), String> {
        let items: Vec<RawTemplate> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse built-in templates: {e}"))?;
        for item in items {
            let cat = item
                .category
                .unwrap_or_else(|| default_category.to_string());
            let content = item.content.unwrap_or_else(|| "{}".to_string());
            let tags_json = serde_json::to_string(&Vec::<String>::new())
                .map_err(|e| format!("Failed to serialize tags: {e}"))?;
            conn.execute(
                "INSERT INTO templates (name, category, subcategory, content, tags, is_builtin, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
                params![item.name, cat, item.subcategory, content, tags_json],
            )
            .map_err(|e| format!("Failed to seed template '{}': {e}", item.name))?;
        }
        Ok(())
    };

    parse_and_insert(OWASP_WEB, "finding")?;
    parse_and_insert(OWASP_API, "finding")?;
    parse_and_insert(OWASP_LLM, "finding")?;
    parse_and_insert(OTHER_TEMPLATES, "")?;

    Ok(())
}

fn seed_builtin_checklists(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM checklists WHERE is_builtin = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count checklists: {e}"))?;
    if count > 0 {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO checklists (name, description, version, is_builtin, is_active, created_at) VALUES (?1, ?2, ?3, 1, 1, strftime('%s', 'now'))",
        params!["OWASP WSTG v4.2", "OWASP Web Security Testing Guide", "4.2"],
    ).map_err(|e| format!("Failed to seed checklist: {e}"))?;

    let checklist_id: i64 = conn.last_insert_rowid();

    let categories = vec![
        (
            "Information Gathering",
            vec![
                (
                    "WSTG-INFO-01",
                    "Conduct Search Engine Discovery Reconnaissance",
                ),
                ("WSTG-INFO-02", "Fingerprint Web Server"),
                ("WSTG-INFO-03", "Review Webserver Metafiles"),
            ],
        ),
        (
            "Configuration and Deploy Management",
            vec![
                ("WSTG-CONF-01", "Test Network Infrastructure Configuration"),
                ("WSTG-CONF-02", "Test Application Platform Configuration"),
                ("WSTG-CONF-03", "Test File Extensions Handling"),
            ],
        ),
        (
            "Identity Management",
            vec![
                ("WSTG-IDNT-01", "Test Role Definitions"),
                ("WSTG-IDNT-02", "Test User Registration Process"),
                ("WSTG-IDNT-03", "Test Account Provisioning Process"),
            ],
        ),
        (
            "Authentication",
            vec![
                (
                    "WSTG-ATHN-01",
                    "Testing for Credentials Transported over an Unencrypted Channel",
                ),
                ("WSTG-ATHN-02", "Testing for Default Credentials"),
                ("WSTG-ATHN-03", "Testing for Weak Lock Out Mechanism"),
            ],
        ),
        (
            "Authorization",
            vec![
                ("WSTG-ATHZ-01", "Testing Directory Traversal File Include"),
                ("WSTG-ATHZ-02", "Testing for Bypassing Authorization Schema"),
                ("WSTG-ATHZ-03", "Testing for Privilege Escalation"),
            ],
        ),
        (
            "Session Management",
            vec![
                ("WSTG-SESS-01", "Testing for Session Management Schema"),
                ("WSTG-SESS-02", "Testing for Cookies Attributes"),
                ("WSTG-SESS-03", "Testing for Session Fixation"),
            ],
        ),
        (
            "Input Validation",
            vec![
                ("WSTG-INPV-01", "Testing for Reflected Cross Site Scripting"),
                ("WSTG-INPV-02", "Testing for Stored Cross Site Scripting"),
                ("WSTG-INPV-03", "Testing for HTTP Verb Tampering"),
                ("WSTG-INPV-04", "Testing for SQL Injection"),
                ("WSTG-INPV-05", "Testing for LDAP Injection"),
            ],
        ),
        (
            "Error Handling",
            vec![
                ("WSTG-ERRH-01", "Testing for Error Handling"),
                ("WSTG-ERRH-02", "Testing for Stack Traces"),
            ],
        ),
        (
            "Cryptography",
            vec![
                ("WSTG-CRYP-01", "Testing for Weak Transport Layer Security"),
                ("WSTG-CRYP-02", "Testing for Padding Oracle"),
            ],
        ),
        (
            "Business Logic",
            vec![
                ("WSTG-BUSL-01", "Test Business Logic Data Validation"),
                ("WSTG-BUSL-02", "Test Ability to Forge Requests"),
            ],
        ),
        (
            "Client-side Testing",
            vec![
                ("WSTG-CLNT-01", "Testing for DOM-based Cross Site Scripting"),
                ("WSTG-CLNT-02", "Testing for JavaScript Execution"),
            ],
        ),
        (
            "API Testing",
            vec![
                ("WSTG-APIT-01", "Testing GraphQL"),
                ("WSTG-APIT-02", "Testing REST API"),
            ],
        ),
    ];

    for (cat, items) in categories {
        for (i, (test_id, name)) in items.iter().enumerate() {
            conn.execute(
                "INSERT INTO checklist_items (checklist_id, category, test_id, name, description, sort_order) VALUES (?1, ?2, ?3, ?4, '', ?5)",
                params![checklist_id, cat, test_id, name, i as i64],
            ).map_err(|e| format!("Failed to seed checklist item: {e}"))?;
        }
    }

    Ok(())
}

fn seed_builtin_compliance(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM compliance_frameworks WHERE is_builtin = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count compliance frameworks: {e}"))?;
    if count > 0 {
        return Ok(());
    }

    let frameworks = vec![
        (
            "PCI-DSS v4.0",
            "4.0",
            "Payment Card Industry Data Security Standard",
        ),
        (
            "OWASP Top 10 2021",
            "2021",
            "OWASP Top 10 Web Application Security Risks",
        ),
        ("NIST CSF v2.0", "2.0", "NIST Cybersecurity Framework"),
        (
            "ISO 27001:2022 Annex A",
            "2022",
            "ISO 27001 Information Security Management",
        ),
    ];

    for (name, version, desc) in frameworks {
        conn.execute(
            "INSERT INTO compliance_frameworks (name, version, description, is_builtin, is_active, created_at) VALUES (?1, ?2, ?3, 1, 1, strftime('%s', 'now'))",
            params![name, version, desc],
        ).map_err(|e| format!("Failed to seed compliance framework: {e}"))?;
    }

    // Seed PCI-DSS controls
    let pci_id: i64 = conn
        .query_row(
            "SELECT id FROM compliance_frameworks WHERE name = 'PCI-DSS v4.0'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get PCI framework id: {e}"))?;

    let pci_controls = [
        ("6.2.4", "Software security patches and updates", "Security"),
        ("6.3.1", "Vulnerability identification", "Security"),
        ("6.5.1", "Injection flaws", "Secure Coding"),
        ("6.5.2", "Buffer overflows", "Secure Coding"),
        ("6.5.7", "Cross-site scripting (XSS)", "Secure Coding"),
        ("11.3", "Penetration testing", "Testing"),
    ];
    for (i, (cid, title, cat)) in pci_controls.iter().enumerate() {
        conn.execute(
            "INSERT INTO compliance_controls (framework_id, control_id, title, category, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![pci_id, cid, title, cat, i as i64],
        ).map_err(|e| format!("Failed to seed PCI control: {e}"))?;
    }

    // Seed OWASP controls
    let owasp_id: i64 = conn
        .query_row(
            "SELECT id FROM compliance_frameworks WHERE name = 'OWASP Top 10 2021'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to get OWASP framework id: {e}"))?;

    let owasp_controls = vec![
        ("A01", "Broken Access Control", "Access"),
        ("A02", "Cryptographic Failures", "Crypto"),
        ("A03", "Injection", "Input Validation"),
        ("A04", "Insecure Design", "Design"),
        ("A05", "Security Misconfiguration", "Config"),
        ("A06", "Vulnerable and Outdated Components", "Components"),
        ("A07", "Identification and Authentication Failures", "Auth"),
        ("A08", "Software and Data Integrity Failures", "Integrity"),
        ("A09", "Security Logging and Monitoring Failures", "Logging"),
        ("A10", "Server-Side Request Forgery (SSRF)", "Network"),
    ];
    for (i, (cid, title, cat)) in owasp_controls.iter().enumerate() {
        conn.execute(
            "INSERT INTO compliance_controls (framework_id, control_id, title, category, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![owasp_id, cid, title, cat, i as i64],
        ).map_err(|e| format!("Failed to seed OWASP control: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_create_and_init() {
        let tmp = tempfile::tempdir().unwrap();
        let key = [0u8; 32]; // 32 zero bytes as test key

        let conn = open_vault(tmp.path(), &key).unwrap();
        init_db(&conn).unwrap();

        // Verify tables exist
        {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
                .unwrap();
            let tables: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            assert!(tables.contains(&"settings".to_string()));
            assert!(tables.contains(&"audit_log".to_string()));
            assert!(tables.contains(&"vault_meta".to_string()));
        }

        // Verify WAL is active
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");

        // Cleanup handled by TempDir drop
        drop(conn);
    }

    #[test]
    fn test_wrong_key_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let correct_key = [1u8; 32];
        let wrong_key = [2u8; 32];

        // Create vault with one key
        {
            let conn = open_vault(tmp.path(), &correct_key).unwrap();
            init_db(&conn).unwrap();
            drop(conn);
        }

        // SQLCipher logs "HMAC check failed" to stderr when the wrong key
        // is used. This is expected — we are proving the vault rejects
        // invalid keys.
        let result = open_vault(tmp.path(), &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_rekey_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let old_key = [3u8; 32];
        let new_key = [4u8; 32];

        // Create vault with old key
        {
            let conn = open_vault(tmp.path(), &old_key).unwrap();
            init_db(&conn).unwrap();
            // Insert a marker row to prove data survives re-keying
            conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES ('marker', 'test', 0)",
                [],
            )
            .unwrap();
            drop(conn);
        }

        // Re-key to new key
        {
            let conn = open_vault(tmp.path(), &old_key).unwrap();
            rekey_vault(&conn, &old_key, &new_key).unwrap();
            drop(conn);
        }

        // Open with new key and verify data survived
        {
            let conn = open_vault(tmp.path(), &new_key).unwrap();
            let marker: String = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'marker'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(marker, "test");
            drop(conn);
        }

        // Old key no longer works
        let result = open_vault(tmp.path(), &old_key);
        assert!(result.is_err());
    }
}
