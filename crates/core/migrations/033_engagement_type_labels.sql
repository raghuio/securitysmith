-- Migration 033: Engagement type labels table
-- User-configurable engagement type list per PROP-033.

CREATE TABLE IF NOT EXISTS engagement_type_labels (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT NOT NULL UNIQUE,
    description TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_engagement_type_labels_active ON engagement_type_labels(is_active);
