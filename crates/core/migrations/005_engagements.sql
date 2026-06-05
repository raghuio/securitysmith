-- Migration 005: Engagements table with scheduling gates
-- Stores engagements (scoped security projects) under clients.
-- Includes credentials_ready, payment_required, payment_cleared for scheduling gates.
-- NOTE: If an old engagements table exists without these columns, drop it manually
--       or recreate the vault. This migration is pre-release.

CREATE TABLE IF NOT EXISTS engagements (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id       INTEGER NOT NULL,
    name            TEXT NOT NULL,
    target_area     TEXT NOT NULL,
    assessment_kind TEXT NOT NULL,
    access_model    TEXT NOT NULL,
    engagement_type TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'scheduled', 'active', 'paused', 'completed')),
    start_date      TEXT,
    end_date        TEXT,
    scope_summary   TEXT,
    objectives      TEXT NOT NULL DEFAULT '[]',
    notes           TEXT,
    tags            TEXT NOT NULL DEFAULT '[]',
    credentials_ready INTEGER NOT NULL DEFAULT 0,
    payment_required  INTEGER NOT NULL DEFAULT 0,
    payment_cleared   INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (client_id) REFERENCES clients(id),
    UNIQUE(client_id, name)
);

CREATE INDEX IF NOT EXISTS idx_engagements_client_id ON engagements(client_id);
CREATE INDEX IF NOT EXISTS idx_engagements_status ON engagements(status);
CREATE INDEX IF NOT EXISTS idx_engagements_active ON engagements(is_active);
CREATE INDEX IF NOT EXISTS idx_engagements_target_area ON engagements(target_area);
CREATE INDEX IF NOT EXISTS idx_engagements_assessment_kind ON engagements(assessment_kind);
CREATE INDEX IF NOT EXISTS idx_engagements_access_model ON engagements(access_model);
CREATE INDEX IF NOT EXISTS idx_engagements_type ON engagements(engagement_type);
