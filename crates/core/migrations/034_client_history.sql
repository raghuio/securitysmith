-- Migration 034: Client history table
-- Field-level change tracking per client per PROP-033.

CREATE TABLE IF NOT EXISTS client_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id   INTEGER NOT NULL,
    field_name  TEXT NOT NULL,
    old_value   TEXT,
    new_value   TEXT,
    changed_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    changed_by  TEXT NOT NULL DEFAULT 'user',
    FOREIGN KEY (client_id) REFERENCES clients(id)
);

CREATE INDEX IF NOT EXISTS idx_client_history_client ON client_history(client_id);
CREATE INDEX IF NOT EXISTS idx_client_history_changed_at ON client_history(changed_at);
