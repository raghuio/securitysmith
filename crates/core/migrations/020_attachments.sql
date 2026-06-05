CREATE TABLE IF NOT EXISTS attachments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type     TEXT NOT NULL
        CHECK (entity_type IN ('finding', 'engagement', 'document')),
    entity_id       INTEGER NOT NULL,
    filename        TEXT NOT NULL,
    original_name   TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    file_size       INTEGER NOT NULL,
    sha256          TEXT NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_attachments_entity ON attachments(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_attachments_active ON attachments(is_active);
