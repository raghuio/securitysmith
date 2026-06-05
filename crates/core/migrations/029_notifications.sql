CREATE TABLE IF NOT EXISTS dismissed_notifications (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    notification_key TEXT NOT NULL UNIQUE,
    dismissed_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_dismissed_key ON dismissed_notifications(notification_key);
