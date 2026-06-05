CREATE TABLE IF NOT EXISTS feeds (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    is_default      INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at      INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS news_articles (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id         INTEGER NOT NULL,
    guid            TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    link            TEXT,
    published_at    INTEGER,
    matched_clients TEXT NOT NULL DEFAULT '[]',
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_guid ON news_articles(feed_id, guid);
CREATE INDEX IF NOT EXISTS idx_articles_feed ON news_articles(feed_id);
CREATE INDEX IF NOT EXISTS idx_articles_published ON news_articles(published_at);
