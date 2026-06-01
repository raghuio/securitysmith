CREATE TABLE IF NOT EXISTS templates (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    category        TEXT NOT NULL
        CHECK (category IN ('finding', 'requirements', 'checklist', 'email', 'status_report', 'engagement_status')),
    subcategory     TEXT NOT NULL DEFAULT '',
    content         TEXT NOT NULL DEFAULT '{}',
    tags            TEXT NOT NULL DEFAULT '[]',
    is_builtin      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_templates_category ON templates(category);
CREATE INDEX IF NOT EXISTS idx_templates_subcategory ON templates(subcategory);
CREATE INDEX IF NOT EXISTS idx_templates_builtin ON templates(is_builtin);
CREATE INDEX IF NOT EXISTS idx_templates_active ON templates(is_active);
