-- Migration 031: Projects table
-- New entity between Client and Engagement per PROP-033.

CREATE TABLE IF NOT EXISTS projects (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id       INTEGER NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'completed', 'archived')),
    start_date      TEXT,
    end_date        TEXT,
    budgeted_hours  INTEGER,
    tech_stack      TEXT NOT NULL DEFAULT '[]',
    tentative_dates TEXT,                      -- JSON array of { label, date, recurring?, recurrence_pattern? }
    tags            TEXT NOT NULL DEFAULT '[]',
    notes           TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (client_id) REFERENCES clients(id),
    UNIQUE(client_id, name)
);

CREATE INDEX IF NOT EXISTS idx_projects_client_id ON projects(client_id);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_active ON projects(is_active);
