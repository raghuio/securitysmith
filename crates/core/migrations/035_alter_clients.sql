-- Migration 035: Alter clients table
-- Enhances clients with business fields per PROP-033.
-- Replaces `name` with `short_name` + `registered_business_name`.
-- Removes `contact_email` in favor of `email`.
-- Drops `tech_stack` (moved to projects per user decision).
-- All existing data is migrated; no data loss.
-- Uses table recreation because SQLite cannot DROP COLUMN on a UNIQUE column.
-- PRAGMA foreign_keys=OFF ensures foreign keys from engagements/projects
-- are not cascade-deleted during the table swap.

PRAGMA foreign_keys = OFF;

CREATE TABLE clients_new (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT,
    short_name               TEXT NOT NULL UNIQUE,
    registered_business_name TEXT,
    country                  TEXT,
    address                  TEXT,
    email                    TEXT,
    contact_number           TEXT,
    business_tier            TEXT
        CHECK (business_tier IN ('enterprise', 'mid_market', 'small')),
    priority                 TEXT
        CHECK (priority IN ('high', 'medium', 'low')),
    status                   TEXT DEFAULT 'active'
        CHECK (status IN ('active', 'inactive', 'prospect')),
    tax_info                 TEXT DEFAULT '{}',
    logo_attachment_id       INTEGER,
    notes                    TEXT,
    tags                     TEXT NOT NULL DEFAULT '[]',
    is_active                INTEGER NOT NULL DEFAULT 1,
    created_at               INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at               INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

INSERT INTO clients_new (
    id, short_name, registered_business_name, country, address, email,
    contact_number, business_tier, priority, status, tax_info, logo_attachment_id,
    notes, tags, is_active, created_at, updated_at
)
SELECT
    id,
    COALESCE(name, 'Unknown')              AS short_name,
    COALESCE(name, '')                     AS registered_business_name,
    ''                                      AS country,
    ''                                      AS address,
    COALESCE(contact_email, '')            AS email,
    ''                                      AS contact_number,
    NULL                                    AS business_tier,
    'medium'                                AS priority,
    'active'                                AS status,
    '{}'                                    AS tax_info,
    NULL                                    AS logo_attachment_id,
    notes,
    COALESCE(tags, '[]')                   AS tags,
    COALESCE(is_active, 1)               AS is_active,
    created_at,
    updated_at
FROM clients;

DROP TABLE clients;
ALTER TABLE clients_new RENAME TO clients;

CREATE INDEX IF NOT EXISTS idx_clients_short_name ON clients(short_name);
CREATE INDEX IF NOT EXISTS idx_clients_status ON clients(status);
CREATE INDEX IF NOT EXISTS idx_clients_active ON clients(is_active);

PRAGMA foreign_keys = ON;
