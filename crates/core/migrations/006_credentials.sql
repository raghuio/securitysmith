CREATE TABLE IF NOT EXISTS credentials (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id   INTEGER NOT NULL,
    label           TEXT NOT NULL,
    credential_type TEXT NOT NULL DEFAULT 'custom',
    value           TEXT NOT NULL,
    notes           TEXT,
    status          TEXT NOT NULL DEFAULT 'not_verified'
        CHECK (status IN ('not_verified', 'working', 'not_working', 'expired')),
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_credentials_engagement_id ON credentials(engagement_id);
CREATE INDEX IF NOT EXISTS idx_credentials_status ON credentials(status);
