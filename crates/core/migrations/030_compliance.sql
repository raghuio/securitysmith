CREATE TABLE IF NOT EXISTS compliance_frameworks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    version     TEXT,
    description TEXT,
    is_builtin  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS compliance_controls (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    framework_id    INTEGER NOT NULL,
    control_id      TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    category        TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (framework_id) REFERENCES compliance_frameworks(id) ON DELETE CASCADE,
    UNIQUE(framework_id, control_id)
);

CREATE TABLE IF NOT EXISTS finding_compliance_mappings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    finding_id      INTEGER NOT NULL,
    control_id      INTEGER NOT NULL,
    notes           TEXT,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (finding_id) REFERENCES findings(id),
    FOREIGN KEY (control_id) REFERENCES compliance_controls(id),
    UNIQUE(finding_id, control_id)
);

CREATE INDEX IF NOT EXISTS idx_controls_framework ON compliance_controls(framework_id);
CREATE INDEX IF NOT EXISTS idx_mappings_finding ON finding_compliance_mappings(finding_id);
CREATE INDEX IF NOT EXISTS idx_mappings_control ON finding_compliance_mappings(control_id);
