CREATE TABLE IF NOT EXISTS time_entries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id   INTEGER NOT NULL,
    entry_date      TEXT NOT NULL,
    hours           REAL NOT NULL CHECK (hours > 0 AND hours <= 24),
    description     TEXT,
    activity_type   TEXT NOT NULL DEFAULT 'testing'
        CHECK (activity_type IN ('testing', 'reporting', 'scoping', 'communication', 'remediation_support', 'retest', 'admin', 'other')),
    is_billable     INTEGER NOT NULL DEFAULT 1,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_time_engagement ON time_entries(engagement_id);
CREATE INDEX IF NOT EXISTS idx_time_date ON time_entries(entry_date);
CREATE INDEX IF NOT EXISTS idx_time_billable ON time_entries(is_billable);

ALTER TABLE engagements ADD COLUMN budgeted_hours REAL;
