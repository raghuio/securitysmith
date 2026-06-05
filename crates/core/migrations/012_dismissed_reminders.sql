CREATE TABLE IF NOT EXISTS dismissed_reminders (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    reminder_key    TEXT NOT NULL UNIQUE,
    dismissed_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_dismissed_key ON dismissed_reminders(reminder_key);
