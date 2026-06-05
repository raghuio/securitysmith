use rusqlite::Connection;
use crate::db;

/// Create a temporary in-memory vault for testing.
/// The vault is initialized with a zeroed key and all migrations applied.
pub fn test_conn() -> Connection {
    let tmp = tempfile::tempdir().unwrap();
    let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
    db::init_db(&conn).unwrap();
    conn
}
