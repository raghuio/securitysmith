CREATE TABLE IF NOT EXISTS documents (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id       INTEGER NOT NULL,
    engagement_id   INTEGER,
    name            TEXT NOT NULL,
    document_type   TEXT NOT NULL
        CHECK (document_type IN ('sow', 'roe', 'nda', 'custom')),
    content         TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'finalized')),
    template_id     INTEGER,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (client_id) REFERENCES clients(id),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_documents_client ON documents(client_id);
CREATE INDEX IF NOT EXISTS idx_documents_engagement ON documents(engagement_id);
CREATE INDEX IF NOT EXISTS idx_documents_type ON documents(document_type);
CREATE INDEX IF NOT EXISTS idx_documents_active ON documents(is_active);
