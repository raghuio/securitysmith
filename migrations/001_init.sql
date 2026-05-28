-- Migration 001: Initial schema for Phase 0 — Vault Shell
-- Tables: settings, audit_log

-- Application settings stored as JSON key-value pairs
CREATE TABLE IF NOT EXISTS settings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    key         TEXT NOT NULL UNIQUE,
    value       TEXT NOT NULL DEFAULT '{}',
    updated_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    created_at  INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Immutable audit trail of every data mutation
CREATE TABLE IF NOT EXISTS audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    table_name  TEXT NOT NULL,
    action      TEXT NOT NULL,        -- INSERT, UPDATE, DELETE
    record_id   TEXT,                 -- Optional reference to affected record
    old_value   TEXT,                 -- JSON snapshot before change
    new_value   TEXT,                 -- JSON snapshot after change
    context     TEXT                  -- Optional: command name, user action, etc.
);

-- Index for fast audit log lookups by table and time
CREATE INDEX IF NOT EXISTS idx_audit_table ON audit_log(table_name);
CREATE INDEX IF NOT EXISTS idx_audit_time ON audit_log(timestamp DESC);
