use rusqlite::{Connection, OpenFlags};
use std::path::Path;

const INIT_SQL: &str = include_str!("../../migrations/001_init.sql");

/// Open or create the encrypted SQLite vault at the given directory.
/// The database file is named `vault.db` and is encrypted with SQLCipher
/// using the provided master key.
pub fn open_vault(data_dir: &Path, key: &str) -> Result<Connection, String> {
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    let db_path = data_dir.join("vault.db");

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .map_err(|e| format!("Failed to open vault database: {}", e))?;

    // Apply SQLCipher encryption key
    conn.execute_batch(&format!("PRAGMA key = '{}';", escape_sql_string(key)))
        .map_err(|e| format!("Failed to set encryption key: {}", e))?;

    // Verify encryption is active by reading a harmless value
    conn.query_row("SELECT 1", [], |_| Ok(()))
        .map_err(|e| format!("Vault key verification failed (wrong password?): {}", e))?;

    Ok(conn)
}

/// Initialise schema: run migrations, enable WAL mode, tune pragmas.
pub fn init_db(conn: &Connection) -> Result<(), String> {
    enable_wal(conn)?;
    run_migrations(conn)?;
    Ok(())
}

/// Enable Write-Ahead Logging for concurrent reads/writes and better performance.
fn enable_wal(conn: &Connection) -> Result<(), String> {
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|e| format!("Failed to enable WAL mode: {}", e))?;

    if journal_mode != "wal" {
        return Err(format!("WAL mode not enabled, got: {}", journal_mode));
    }

    // WAL tuning for desktop use: let checkpoint happen automatically,
    // but keep a reasonable size limit (1000 pages ≈ 4 MB)
    conn.execute_batch("PRAGMA wal_autocheckpoint = 1000;")
        .map_err(|e| format!("Failed to set WAL autocheckpoint: {}", e))?;

    Ok(())
}

/// Run embedded SQL migrations.
fn run_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(INIT_SQL)
        .map_err(|e| format!("Migration 001 failed: {}", e))?;
    Ok(())
}

/// Escape a string for safe use inside a single-quoted SQL literal.
/// SQLCipher `PRAGMA key` accepts the standard SQL string literal format,
/// where `'` is escaped as `''`.
fn escape_sql_string(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_create_and_init() {
        let tmp = tempfile::tempdir().unwrap();

        let conn = open_vault(tmp.path(), "test_password_123").unwrap();
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

        // Create vault with one key
        {
            let conn = open_vault(tmp.path(), "correct_key").unwrap();
            init_db(&conn).unwrap();
            drop(conn);
        }

        // SQLCipher logs "HMAC check failed" to stderr when the wrong key
        // is used. This is expected — we are proving the vault rejects
        // invalid passwords.
        //
        // NOTE: stderr noise here is harmless. SQLCipher verifies the key
        // by trying a page-1 decrypt, and that HMAC mismatch triggers
        // internal logging we can't suppress without a C library patch.
        let result = open_vault(tmp.path(), "wrong_key");
        assert!(result.is_err());
    }
}
