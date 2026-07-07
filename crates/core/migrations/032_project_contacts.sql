-- Migration 032: Project contacts table
-- Mirrors client_contacts but scoped to project_id per PROP-033.

CREATE TABLE IF NOT EXISTS project_contacts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL,
    name        TEXT NOT NULL,
    email       TEXT NOT NULL,
    phone       TEXT,
    role        TEXT NOT NULL DEFAULT 'other'
        CHECK (role IN ('technical_poc', 'management', 'billing', 'legal', 'remediation', 'executive', 'other')),
    role_label  TEXT,
    title       TEXT,
    notes       TEXT,
    is_primary  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_project_contacts_project ON project_contacts(project_id);
CREATE INDEX IF NOT EXISTS idx_project_contacts_active ON project_contacts(is_active);
