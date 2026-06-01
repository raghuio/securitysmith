CREATE TABLE IF NOT EXISTS checklists (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    version     TEXT,
    is_builtin  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS checklist_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    checklist_id    INTEGER NOT NULL,
    category        TEXT NOT NULL,
    test_id         TEXT,
    name            TEXT NOT NULL,
    description     TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (checklist_id) REFERENCES checklists(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS engagement_checklist_items (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id       INTEGER NOT NULL,
    checklist_item_id   INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'not_started'
        CHECK (status IN ('not_started', 'in_progress', 'tested', 'not_applicable', 'finding_created', 'deferred')),
    linked_finding_id   INTEGER,
    notes               TEXT,
    updated_at          INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id),
    FOREIGN KEY (checklist_item_id) REFERENCES checklist_items(id),
    FOREIGN KEY (linked_finding_id) REFERENCES findings(id),
    UNIQUE(engagement_id, checklist_item_id)
);

CREATE INDEX IF NOT EXISTS idx_checklist_items_checklist ON checklist_items(checklist_id);
CREATE INDEX IF NOT EXISTS idx_engagement_checklists_engagement ON engagement_checklist_items(engagement_id);
