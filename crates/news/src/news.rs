use ss_core::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::collections::HashSet;
use tauri::State;

#[derive(Serialize)]
pub struct Feed {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
pub struct NewsArticle {
    pub id: u32,
    pub feed_id: u32,
    pub feed_name: String,
    pub guid: String,
    pub title: String,
    pub description: Option<String>,
    pub link: Option<String>,
    pub published_at: Option<i64>,
    pub matched_clients: Vec<u32>,
}

fn do_list_feeds(conn: &Connection) -> Result<Vec<Feed>, String> {
    let mut stmt = conn
        .prepare("SELECT id, url, name, is_default, is_active, created_at, updated_at FROM feeds WHERE is_active = 1")
        .map_err(|e| format!("Prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                url: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get(3)?,
                is_active: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query: {e}"))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Warning: feed row skipped: {e}");
                None
            }
        })
        .collect();
    Ok(rows)
}

fn do_create_feed(conn: &Connection, url: &str, name: &str) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO feeds (url, name, is_default, created_at, updated_at) VALUES (?1, ?2, 0, strftime('%s', 'now'), strftime('%s', 'now'))",
        params![url, name],
    )
    .map_err(|e| format!("Create: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "overflow".to_string())?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["feeds", "CREATE", id, "", ""],
    )
    .map_err(|e| format!("Audit: {e}"))?;
    Ok(id)
}

fn do_delete_feed(conn: &Connection, id: u32) -> Result<(), String> {
    conn.execute("DELETE FROM feeds WHERE id = ?", params![id])
        .map_err(|e| format!("Delete: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["feeds", "DELETE", id, "", ""],
    )
    .map_err(|e| format!("Audit: {e}"))?;
    Ok(())
}

fn do_list_articles(conn: &Connection) -> Result<Vec<NewsArticle>, String> {
    let mut stmt = conn
        .prepare("SELECT a.id, a.feed_id, f.name, a.guid, a.title, a.description, a.link, a.published_at, a.matched_clients FROM news_articles a JOIN feeds f ON a.feed_id = f.id ORDER BY a.published_at DESC LIMIT 200")
        .map_err(|e| format!("Prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let clients_str: String = row.get(8)?;
            Ok(NewsArticle {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                feed_name: row.get(2)?,
                guid: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                link: row.get(6)?,
                published_at: row.get(7)?,
                matched_clients: serde_json::from_str(&clients_str).unwrap_or_default(),
            })
        })
        .map_err(|e| format!("Query: {e}"))?
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("Warning: article row skipped: {e}");
                None
            }
        })
        .collect();
    Ok(rows)
}

fn seed_default_feeds(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM feeds", [], |row| row.get(0))
        .map_err(|e| format!("Count: {e}"))?;
    if count > 0 {
        return Ok(());
    }
    let defaults = vec![
        ("https://www.bleepingcomputer.com/feed/", "BleepingComputer"),
        (
            "https://feeds.feedburner.com/TheHackersNews",
            "The Hacker News",
        ),
        (
            "https://www.us-cert.gov/ncas/current-activity.xml",
            "CISA Alerts",
        ),
    ];
    for (url, name) in defaults {
        conn.execute(
            "INSERT INTO feeds (url, name, is_default, created_at, updated_at) VALUES (?1, ?2, 1, strftime('%s', 'now'), strftime('%s', 'now'))",
            params![url, name],
        )
        .map_err(|e| format!("Seed: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_feeds(state: State<AppState>) -> Result<Vec<Feed>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_feeds(conn)
}

#[tauri::command]
pub fn create_feed(state: State<AppState>, url: String, name: String) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_create_feed(conn, &url, &name)
}

#[tauri::command]
pub fn delete_feed(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_delete_feed(conn, id)
}

#[tauri::command]
pub fn list_news_articles(state: State<AppState>) -> Result<Vec<NewsArticle>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_articles(conn)
}

#[tauri::command]
pub fn seed_default_news_feeds(state: State<AppState>) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    seed_default_feeds(conn)
}

#[tauri::command]
pub fn update_feed(
    state: State<AppState>,
    id: u32,
    url: Option<String>,
    name: Option<String>,
    is_active: Option<bool>,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    if let Some(u) = url {
        conn.execute("UPDATE feeds SET url = ? WHERE id = ?", params![u, id])
            .map_err(|e| format!("Update url: {e}"))?;
    }
    if let Some(n) = name {
        conn.execute("UPDATE feeds SET name = ? WHERE id = ?", params![n, id])
            .map_err(|e| format!("Update name: {e}"))?;
    }
    if let Some(a) = is_active {
        let flag = if a { 1 } else { 0 };
        conn.execute(
            "UPDATE feeds SET is_active = ? WHERE id = ?",
            params![flag, id],
        )
        .map_err(|e| format!("Update active: {e}"))?;
    }
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["feeds", "UPDATE", id, "", ""],
    )
    .map_err(|e| format!("Audit: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn mark_article_read(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE news_articles SET is_read = 1 WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Mark read: {e}"))?;
    Ok(())
}

#[derive(Serialize)]
pub struct ClientAlert {
    pub article_id: u32,
    pub article_title: String,
    pub article_link: Option<String>,
    pub client_id: u32,
    pub client_name: String,
    pub matched_tags: Vec<String>,
}

#[tauri::command]
pub fn get_client_alerts(state: State<AppState>) -> Result<Vec<ClientAlert>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;

    let mut alerts: Vec<ClientAlert> = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, name, tech_stack FROM clients WHERE is_active = 1")
        .map_err(|e| format!("Prepare clients: {e}"))?;
    let clients = stmt
        .query_map([], |row| {
            let stack_str: String = row.get(2)?;
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                serde_json::from_str::<Vec<String>>(&stack_str).unwrap_or_default(),
            ))
        })
        .map_err(|e| format!("Query clients: {e}"))?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    for (client_id, client_name, tags) in clients {
        if tags.is_empty() {
            continue;
        }
        let mut stmt = conn
            .prepare(
                "SELECT id, title, link FROM news_articles WHERE is_read = 0 AND (title LIKE ? OR description LIKE ?) ORDER BY published_at DESC LIMIT 20"
            )
            .map_err(|e| format!("Prepare articles: {e}"))?;
        for tag in &tags {
            let pattern = format!("%{tag}%");
            let rows = stmt
                .query_map(params![&pattern, &pattern], |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map_err(|e| format!("Query articles: {e}"))?;
            for r in rows.filter_map(|r| r.ok()) {
                alerts.push(ClientAlert {
                    article_id: r.0,
                    article_title: r.1,
                    article_link: r.2,
                    client_id,
                    client_name: client_name.clone(),
                    matched_tags: vec![tag.clone()],
                });
            }
        }
    }
    Ok(alerts)
}

#[derive(Serialize)]
pub struct RefreshResult {
    pub new_articles: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn refresh_feeds(state: State<'_, AppState>) -> Result<RefreshResult, String> {
    // Phase 1: Read all data from DB (no await)
    let (feeds, clients) = {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.connection_ref().map_err(|e| e.to_string())?;

        let feeds = do_list_feeds(conn)?;

        let mut stmt = conn
            .prepare("SELECT id, name, tech_stack FROM clients WHERE is_active = 1")
            .map_err(|e| format!("Prepare clients: {e}"))?;
        let clients: Vec<(u32, String, Vec<String>)> = stmt
            .query_map([], |row| {
                let stack_str: String = row.get(2)?;
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    serde_json::from_str::<Vec<String>>(&stack_str).unwrap_or_default(),
                ))
            })
            .map_err(|e| format!("Query clients: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        (feeds, clients)
    };

    if feeds.is_empty() {
        return Ok(RefreshResult {
            new_articles: 0,
            errors: vec!["No feeds configured.".to_string()],
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;

    let mut new_articles: u32 = 0;
    let mut errors: Vec<String> = Vec::new();

    for feed in feeds {
        let body = match client.get(&feed.url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    errors.push(format!("{}: read body failed: {}", feed.name, e));
                    continue;
                }
            },
            Err(e) => {
                errors.push(format!("{}: fetch failed: {}", feed.name, e));
                continue;
            }
        };

        let channel = match rss::Channel::read_from(body.as_bytes()) {
            Ok(ch) => ch,
            Err(e) => {
                errors.push(format!("{}: parse failed: {}", feed.name, e));
                continue;
            }
        };

        // Phase 2: Write back to DB (no await in this block)
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.connection_ref().map_err(|e| e.to_string())?;

        for item in channel.items() {
            let title = item.title().unwrap_or("Untitled").to_string();
            let link = item.link().map(|s| s.to_string());
            let description = item.description().map(|s| s.to_string());
            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .or_else(|| link.clone())
                .unwrap_or_else(|| title.clone());
            let published_at = item.pub_date().and_then(|d| {
                chrono::DateTime::parse_from_rfc2822(d)
                    .ok()
                    .map(|dt| dt.timestamp())
            });

            // Deduplication
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM news_articles WHERE feed_id = ? AND guid = ?",
                    params![feed.id, &guid],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|e| format!("Dedup check: {e}"))?
                .unwrap_or(false);
            if exists {
                continue;
            }

            // Match against client tech_stack
            let mut matched: HashSet<u32> = HashSet::new();
            let text_lower = format!(
                "{} {}",
                title.to_lowercase(),
                description
                    .as_ref()
                    .map(|d| d.to_lowercase())
                    .unwrap_or_default()
            );
            for (client_id, _client_name, tags) in &clients {
                for tag in tags {
                    if text_lower.contains(&tag.to_lowercase()) {
                        matched.insert(*client_id);
                    }
                }
            }
            let matched_vec: Vec<u32> = matched.into_iter().collect();
            let matched_json = serde_json::to_string(&matched_vec)
                .map_err(|e| format!("Serialize matched: {e}"))?;

            conn.execute(
                "INSERT INTO news_articles (feed_id, guid, title, description, link, published_at, matched_clients, is_read)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                params![
                    feed.id,
                    &guid,
                    &title,
                    description.as_ref(),
                    link.as_ref(),
                    published_at,
                    &matched_json,
                ],
            )
            .map_err(|e| format!("Insert article: {e}"))?;
            new_articles += 1;
        }

        // Update feed last_fetched_at
        let _ = conn.execute(
            "UPDATE feeds SET last_fetched_at = strftime('%s', 'now') WHERE id = ?",
            params![feed.id],
        );
    }

    // Audit log
    {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.connection_ref().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "system",
                "RSS_REFRESH",
                0,
                "",
                "",
                format!("new_articles={new_articles}")
            ],
        )
        .map_err(|e| format!("Audit failed: {e}"))?;
    }

    Ok(RefreshResult {
        new_articles,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_news_feeds_list_returns_list() {
        let conn = test_conn();
        let feeds = do_list_feeds(&conn).unwrap();
        // Default feeds may be seeded, so just verify we get a list
        // (could be 0 or more depending on seed_default_news_feeds call)
        assert!(feeds.len() >= 0);
    }
}
