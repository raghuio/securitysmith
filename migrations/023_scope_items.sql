CREATE TABLE IF NOT EXISTS scope_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    engagement_id   INTEGER NOT NULL,
    item_type       TEXT NOT NULL
        CHECK (item_type IN ('url', 'ip', 'ip_range', 'cidr', 'domain', 'subdomain', 'application', 'api_endpoint', 'host', 'other')),
    value           TEXT NOT NULL,
    is_in_scope     INTEGER NOT NULL DEFAULT 1,
    environment     TEXT,
    notes           TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE INDEX IF NOT EXISTS idx_scope_engagement ON scope_items(engagement_id);
CREATE INDEX IF NOT EXISTS idx_scope_type ON scope_items(item_type);
