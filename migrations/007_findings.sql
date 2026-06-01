CREATE TABLE IF NOT EXISTS findings (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id       INTEGER NOT NULL,
    title               TEXT NOT NULL,
    severity            TEXT NOT NULL
        CHECK (severity IN ('critical', 'high', 'medium', 'low', 'informational')),
    cvss_score          REAL,
    owasp_category      TEXT,
    cwe_id              TEXT,
    overview            TEXT NOT NULL,
    summary             TEXT NOT NULL,
    affected_endpoints  TEXT NOT NULL DEFAULT '[]',
    evidence            TEXT NOT NULL DEFAULT '[]',
    impact_items        TEXT NOT NULL DEFAULT '[]',
    remediation_items   TEXT NOT NULL DEFAULT '[]',
    steps_to_reproduce  TEXT NOT NULL,
    references_json     TEXT NOT NULL DEFAULT '[]',
    status              TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'confirmed', 'reported', 'fixed', 'accepted', 'false_positive', 'wont_fix')),
    tags                TEXT NOT NULL DEFAULT '[]',
    notes               TEXT,
    is_active           INTEGER NOT NULL DEFAULT 1,
    created_at          INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at          INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_findings_engagement_id ON findings(engagement_id);
CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
CREATE INDEX IF NOT EXISTS idx_findings_status ON findings(status);
CREATE INDEX IF NOT EXISTS idx_findings_owasp ON findings(owasp_category);
CREATE INDEX IF NOT EXISTS idx_findings_active ON findings(is_active);
CREATE INDEX IF NOT EXISTS idx_findings_created ON findings(created_at);
