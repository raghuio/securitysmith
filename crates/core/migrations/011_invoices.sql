CREATE TABLE IF NOT EXISTS invoices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id       INTEGER NOT NULL,
    engagement_id   INTEGER,
    document_type   TEXT NOT NULL
        CHECK (document_type IN ('quote', 'invoice')),
    invoice_number  TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'sent', 'paid', 'cancelled', 'overdue')),
    subtotal_cents  INTEGER NOT NULL DEFAULT 0,
    tax_rate_bps    INTEGER NOT NULL DEFAULT 0,
    discount_cents  INTEGER NOT NULL DEFAULT 0,
    discount_pct_bps INTEGER NOT NULL DEFAULT 0,
    total_cents     INTEGER NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'USD',
    notes           TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (client_id) REFERENCES clients(id),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id)
);

CREATE TABLE IF NOT EXISTS invoice_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id      INTEGER NOT NULL,
    description     TEXT NOT NULL,
    quantity        INTEGER NOT NULL DEFAULT 1,
    rate_cents      INTEGER NOT NULL DEFAULT 0,
    total_cents     INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_invoices_client ON invoices(client_id);
CREATE INDEX IF NOT EXISTS idx_invoices_engagement ON invoices(engagement_id);
CREATE INDEX IF NOT EXISTS idx_invoices_number ON invoices(invoice_number);
CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_active ON invoices(is_active);
