CREATE TABLE IF NOT EXISTS vault_meta (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    key_salt BLOB NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
