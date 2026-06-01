CREATE TABLE IF NOT EXISTS reports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id   INTEGER NOT NULL,
    name            TEXT NOT NULL,
    executive_summary TEXT NOT NULL DEFAULT '',
    appendix        TEXT NOT NULL DEFAULT '',
    included_finding_ids TEXT NOT NULL DEFAULT '[]',
    status          TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'generated')),
    generated_at    INTEGER,
    file_path       TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_reports_engagement ON reports(engagement_id);
CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status);
CREATE INDEX IF NOT EXISTS idx_reports_active ON reports(is_active);
