CREATE TABLE IF NOT EXISTS client_contacts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id   INTEGER NOT NULL,
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
    FOREIGN KEY (client_id) REFERENCES clients(id)
);

CREATE INDEX IF NOT EXISTS idx_contacts_client ON client_contacts(client_id);
CREATE INDEX IF NOT EXISTS idx_contacts_active ON client_contacts(is_active);
