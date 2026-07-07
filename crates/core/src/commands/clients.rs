use crate::error::AppError;
use crate::ids::ClientId;
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Client {
    pub id: ClientId,
    pub short_name: String,
    pub registered_business_name: String,
    pub country: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub contact_number: Option<String>,
    pub business_tier: Option<String>,
    pub priority: Option<String>,
    pub status: String,
    pub tax_info: Option<String>,
    pub logo_attachment_id: Option<u32>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ClientHistoryEntry {
    pub id: u32,
    pub client_id: u32,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_at: i64,
    pub changed_by: String,
}

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub client_count: u32,
    pub finding_count: u32,
    pub engagement_count: u32,
    pub project_count: u32,
    pub findings_ready: bool,
    pub engagements_ready: bool,
}

fn parse_json_arr(tag_str: &str) -> Vec<String> {
    serde_json::from_str(tag_str).unwrap_or_default()
}

fn row_to_client(row: &rusqlite::Row) -> Result<Client, rusqlite::Error> {
    let tags_str: String = row.get(13)?;
    Ok(Client {
        id: row.get(0)?,
        short_name: row.get(1)?,
        registered_business_name: row.get(2)?,
        country: row.get(3)?,
        address: row.get(4)?,
        email: row.get(5)?,
        contact_number: row.get(6)?,
        business_tier: row.get(7)?,
        priority: row.get(8)?,
        status: row.get(9)?,
        tax_info: row.get(10)?,
        logo_attachment_id: row.get(11)?,
        notes: row.get(12)?,
        tags: parse_json_arr(&tags_str),
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn row_to_history(row: &rusqlite::Row) -> Result<ClientHistoryEntry, rusqlite::Error> {
    Ok(ClientHistoryEntry {
        id: row.get(0)?,
        client_id: row.get(1)?,
        field_name: row.get(2)?,
        old_value: row.get(3)?,
        new_value: row.get(4)?,
        changed_at: row.get(5)?,
        changed_by: row.get(6)?,
    })
}

// ─────────────────────────────────────────────────────────────
// Core logic (testable without Tauri State)
// ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn do_create_client(
    conn: &Connection,
    short_name: &str,
    registered_business_name: &str,
    country: Option<&str>,
    address: Option<&str>,
    email: Option<&str>,
    contact_number: Option<&str>,
    business_tier: Option<&str>,
    priority: Option<&str>,
    status: Option<&str>,
    tax_info: Option<&str>,
    logo_attachment_id: Option<u32>,
    tags: Option<&Vec<String>>,
    notes: Option<&str>,
) -> crate::error::Result<u32> {
    let tags_json = serde_json::to_string(&tags.cloned().unwrap_or_default())
        .map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO clients (short_name, registered_business_name, country, address, email, contact_number, business_tier, priority, status, tax_info, logo_attachment_id, notes, tags, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, strftime('%s', 'now'))",
        params![
            short_name, registered_business_name, country, address, email,
            contact_number, business_tier, priority,
            status.unwrap_or("active"), tax_info, logo_attachment_id, notes, tags_json,
        ],
    )
    .map_err(AppError::from)?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;
    let new_client = do_get_client(conn, id).map_err(|e| e.to_string())?;
    let new_json = serde_json::to_string(&new_client)
        .map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &new_json,
            "create_client command"
        ],
    )
    .map_err(AppError::from)?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "client", id)
        .map_err(AppError::from)?;

    Ok(id)
}

fn do_get_client(conn: &Connection, id: u32) -> Result<Client, AppError> {
    let client: Option<Client> = conn
        .query_row(
            "SELECT id, short_name, registered_business_name, country, address, email, contact_number,
                    business_tier, priority, status, tax_info, logo_attachment_id, notes, tags,
                    created_at, updated_at
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(AppError::from)?;

    client.ok_or(AppError::ClientNotFound(id))
}

fn do_get_client_history(conn: &Connection, client_id: u32) -> crate::error::Result<Vec<ClientHistoryEntry>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, client_id, field_name, old_value, new_value, changed_at, changed_by
             FROM client_history WHERE client_id = ?1 ORDER BY changed_at DESC"
        )
        .map_err(AppError::from)?;
    let rows: Vec<ClientHistoryEntry> = stmt
        .query_map(params![client_id], row_to_history)
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    Ok(rows)
}

fn write_client_history(
    conn: &Connection,
    client_id: u32,
    field_name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> crate::error::Result<()> {
    conn.execute(
        "INSERT INTO client_history (client_id, field_name, old_value, new_value, changed_at, changed_by)
         VALUES (?1, ?2, ?3, ?4, strftime('%s', 'now'), 'user')",
        params![client_id, field_name, old_value, new_value],
    )
    .map_err(AppError::from)?;
    Ok(())
}

#[allow(clippy::collapsible_if, clippy::op_ref)]
fn do_update_client(
    conn: &Connection,
    id: u32,
    short_name: Option<&str>,
    registered_business_name: Option<&str>,
    country: Option<&str>,
    address: Option<&str>,
    email: Option<&str>,
    contact_number: Option<&str>,
    business_tier: Option<&str>,
    priority: Option<&str>,
    status: Option<&str>,
    tax_info: Option<&str>,
    logo_attachment_id: Option<u32>,
    tags: Option<&Vec<String>>,
    notes: Option<&str>,
) -> crate::error::Result<()> {
    let old: Option<Client> = conn
        .query_row(
            "SELECT id, short_name, registered_business_name, country, address, email, contact_number,
                    business_tier, priority, status, tax_info, logo_attachment_id, notes, tags,
                    created_at, updated_at
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(AppError::from)?;

    let old = old.ok_or(AppError::ClientNotFound(id))?;

    let update_short = short_name.unwrap_or(&old.short_name);
    let update_registered = registered_business_name.unwrap_or(&old.registered_business_name);
    let update_country = country.or(old.country.as_deref());
    let update_address = address.or(old.address.as_deref());
    let update_email = email.or(old.email.as_deref());
    let update_contact = contact_number.or(old.contact_number.as_deref());
    let update_tier = business_tier.or(old.business_tier.as_deref());
    let update_priority = priority.or(old.priority.as_deref());
    let update_status = status.unwrap_or(&old.status);
    let update_tax = tax_info.or(old.tax_info.as_deref());
    let update_logo = logo_attachment_id.or(old.logo_attachment_id);
    let update_notes = notes.or(old.notes.as_deref());
    let update_tags = tags.cloned().unwrap_or(old.tags.clone());
    let tags_json = serde_json::to_string(&update_tags)
        .map_err(AppError::from)?;

    conn.execute(
        "UPDATE clients SET
            short_name = ?1,
            registered_business_name = ?2,
            country = ?3,
            address = ?4,
            email = ?5,
            contact_number = ?6,
            business_tier = ?7,
            priority = ?8,
            status = ?9,
            tax_info = ?10,
            logo_attachment_id = ?11,
            notes = ?12,
            tags = ?13,
            updated_at = strftime('%s', 'now')
         WHERE id = ?14",
        params![
            update_short, update_registered, update_country, update_address,
            update_email, update_contact, update_tier, update_priority,
            update_status, update_tax, update_logo, update_notes, tags_json, id
        ],
    )
    .map_err(AppError::from)?;

    // Write field-level history for changed fields
    if let Some(s) = short_name {
        if s != &old.short_name {
            write_client_history(conn, id, "short_name", Some(old.short_name.as_str()), Some(s))?;
        }
    }
    if let Some(s) = registered_business_name {
        if s != &old.registered_business_name {
            write_client_history(conn, id, "registered_business_name", Some(old.registered_business_name.as_str()), Some(s))?;
        }
    }
    if let Some(s) = country {
        if Some(s) != old.country.as_deref() {
            write_client_history(conn, id, "country", old.country.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = address {
        if Some(s) != old.address.as_deref() {
            write_client_history(conn, id, "address", old.address.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = email {
        if Some(s) != old.email.as_deref() {
            write_client_history(conn, id, "email", old.email.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = contact_number {
        if Some(s) != old.contact_number.as_deref() {
            write_client_history(conn, id, "contact_number", old.contact_number.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = business_tier {
        if Some(s) != old.business_tier.as_deref() {
            write_client_history(conn, id, "business_tier", old.business_tier.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = priority {
        if Some(s) != old.priority.as_deref() {
            write_client_history(conn, id, "priority", old.priority.as_deref(), Some(s))?;
        }
    }
    if let Some(s) = status {
        if s != &old.status {
            write_client_history(conn, id, "status", Some(old.status.as_str()), Some(s))?;
        }
    }
    if let Some(s) = tax_info {
        if Some(s) != old.tax_info.as_deref() {
            write_client_history(conn, id, "tax_info", old.tax_info.as_deref(), Some(s))?;
        }
    }
    if logo_attachment_id != old.logo_attachment_id {
        write_client_history(
            conn, id, "logo_attachment_id",
            old.logo_attachment_id.map(|v| v.to_string()).as_deref(),
            logo_attachment_id.map(|v| v.to_string()).as_deref()
        )?;
    }
    if let Some(s) = notes {
        if Some(s) != old.notes.as_deref() {
            write_client_history(conn, id, "notes", old.notes.as_deref(), Some(s))?;
        }
    }

    let old_json = serde_json::to_string(&old)
        .map_err(AppError::from)?;
    let new_client = do_get_client(conn, id).map_err(|e| e.to_string())?;
    let new_json = serde_json::to_string(&new_client)
        .map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_client command"
        ],
    )
    .map_err(AppError::from)?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "client", id)
        .map_err(AppError::from)?;

    Ok(())
}

fn do_delete_client(conn: &Connection, id: u32) -> crate::error::Result<()> {
    let old: Option<Client> = conn
        .query_row(
            "SELECT id, short_name, registered_business_name, country, address, email, contact_number,
                    business_tier, priority, status, tax_info, logo_attachment_id, notes, tags,
                    created_at, updated_at
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(AppError::from)?;

    let old = old.ok_or(AppError::ClientNotFound(id))?;

    conn.execute(
        "UPDATE clients SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?1",
        params![id],
    )
    .map_err(AppError::from)?;

    let old_json = serde_json::to_string(&old)
        .map_err(AppError::from)?;
    let new_json = serde_json::json!({
        "id": id,
        "is_active": 0
    })
    .to_string();

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "DELETE",
            &id.to_string(),
            &old_json,
            &new_json,
            "delete_client command"
        ],
    )
    .map_err(AppError::from)?;

    // Remove from global search index (PROP-028)
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'client' AND entity_id = ?1",
        params![id],
    )
    .map_err(AppError::from)?;

    Ok(())
}

fn do_list_clients(conn: &Connection, search: Option<&str>) -> crate::error::Result<Vec<Client>> {
    let mut sql = String::from(
        "SELECT id, short_name, registered_business_name, country, address, email, contact_number,
                business_tier, priority, status, tax_info, logo_attachment_id, notes, tags,
                created_at, updated_at
         FROM clients WHERE is_active = 1",
    );

    let results = if let Some(s) = search {
        let pattern = format!("%{}%", s.trim());
        sql.push_str(
            " AND (short_name LIKE ?1 OR registered_business_name LIKE ?1 OR email LIKE ?1 OR country LIKE ?1 OR tags LIKE ?1)",
        );
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(AppError::from)?;
        stmt.query_map(params![pattern], row_to_client)
            .map_err(AppError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)?
    } else {
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(AppError::from)?;
        stmt.query_map([], row_to_client)
            .map_err(AppError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from)?
    };

    Ok(results)
}

fn do_get_dashboard_stats(conn: &Connection) -> crate::error::Result<DashboardStats> {
    let client_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM clients WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let project_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE is_active = 1 AND status = 'active'",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let engagement_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM engagements WHERE is_active = 1 AND status = 'active'",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let finding_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    Ok(DashboardStats {
        client_count,
        finding_count,
        engagement_count,
        project_count,
        findings_ready: true,
        engagements_ready: true,
    })
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

fn validate_short_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Client short name is required.".to_string()));
    }
    if name.len() > 100 {
        return Err(AppError::Validation("Client short name must be 100 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_registered_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("Registered business name is required.".to_string()));
    }
    if name.len() > 255 {
        return Err(AppError::Validation("Registered business name must be 255 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), AppError> {
    if !email.is_empty() && !is_valid_email(email) {
        return Err(AppError::Validation("Email address is invalid.".to_string()));
    }
    Ok(())
}

fn validate_notes(notes: &str) -> Result<(), AppError> {
    if notes.len() > 10_000 {
        return Err(AppError::Validation("Notes must be 10,000 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_country(country: &str) -> Result<(), AppError> {
    if country.len() > 100 {
        return Err(AppError::Validation("Country must be 100 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_address(address: &str) -> Result<(), AppError> {
    if address.len() > 2000 {
        return Err(AppError::Validation("Address must be 2,000 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_contact_number(phone: &str) -> Result<(), AppError> {
    if phone.len() > 50 {
        return Err(AppError::Validation("Contact number must be 50 characters or fewer.".to_string()));
    }
    Ok(())
}

fn validate_tax_info(tax: &str) -> Result<(), AppError> {
    if tax.len() > 4000 {
        return Err(AppError::Validation("Tax info must be 4,000 characters or fewer.".to_string()));
    }
    // Validate it's valid JSON (or empty)
    if !tax.is_empty() && tax != "{}" {
        serde_json::from_str::<serde_json::Value>(tax)
            .map_err(|_| AppError::Validation("Tax information is not valid JSON.".to_string()))?;
    }
    Ok(())
}

#[tauri::command]
/// Create a new client.
pub fn create_client(
    state: State<AppState>,
    short_name: String,
    registered_business_name: String,
    country: Option<String>,
    address: Option<String>,
    email: Option<String>,
    contact_number: Option<String>,
    business_tier: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    tax_info: Option<String>,
    logo_attachment_id: Option<u32>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
) -> Result<u32, String> {
    validate_short_name(&short_name)?;
    validate_registered_name(&registered_business_name)?;
    if let Some(ref e) = email { validate_email(e)?; }
    if let Some(ref n) = notes { validate_notes(n)?; }
    if let Some(ref c) = country { validate_country(c)?; }
    if let Some(ref a) = address { validate_address(a)?; }
    if let Some(ref p) = contact_number { validate_contact_number(p)?; }
    if let Some(ref t) = tax_info { validate_tax_info(t)?; }

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_create_client(
        conn,
        short_name.trim(),
        registered_business_name.trim(),
        country.as_deref(),
        address.as_deref(),
        email.as_deref(),
        contact_number.as_deref(),
        business_tier.as_deref(),
        priority.as_deref(),
        status.as_deref(),
        tax_info.as_deref(),
        logo_attachment_id,
        tags.as_ref(),
        notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
/// Retrieve a client by ID.
pub fn get_client(state: State<AppState>, id: u32) -> Result<Client, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_get_client(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
/// Retrieve client field-level history.
pub fn get_client_history(state: State<AppState>, client_id: u32) -> Result<Vec<ClientHistoryEntry>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_get_client_history(conn, client_id).map_err(|e| e.to_string())
}

#[tauri::command]
/// Update an existing client.
pub fn update_client(
    state: State<AppState>,
    id: u32,
    short_name: Option<String>,
    registered_business_name: Option<String>,
    country: Option<String>,
    address: Option<String>,
    email: Option<String>,
    contact_number: Option<String>,
    business_tier: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    tax_info: Option<String>,
    logo_attachment_id: Option<u32>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
) -> Result<(), String> {
    if let Some(ref n) = short_name { validate_short_name(n)?; }
    if let Some(ref n) = registered_business_name { validate_registered_name(n)?; }
    if let Some(ref e) = email { validate_email(e)?; }
    if let Some(ref n) = notes { validate_notes(n)?; }
    if let Some(ref c) = country { validate_country(c)?; }
    if let Some(ref a) = address { validate_address(a)?; }
    if let Some(ref p) = contact_number { validate_contact_number(p)?; }
    if let Some(ref t) = tax_info { validate_tax_info(t)?; }

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_update_client(
        conn, id,
        short_name.as_deref(),
        registered_business_name.as_deref(),
        country.as_deref(),
        address.as_deref(),
        email.as_deref(),
        contact_number.as_deref(),
        business_tier.as_deref(),
        priority.as_deref(),
        status.as_deref(),
        tax_info.as_deref(),
        logo_attachment_id,
        tags.as_ref(),
        notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
/// Delete a client by ID.
pub fn delete_client(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_delete_client(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
/// List all clients with optional search.
pub fn list_clients(state: State<AppState>, search: Option<String>) -> Result<Vec<Client>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_list_clients(conn, search.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
/// Get dashboard statistics.
pub fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_get_dashboard_stats(conn).map_err(|e| e.to_string())
}

fn is_valid_email(email: &str) -> bool {
    use validator::ValidateEmail;
    email.validate_email()
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;
    use crate::db;

    #[test]
    fn test_create_and_get_client() {
        let conn = test_conn();
        let id = do_create_client(
            &conn,
            "Acme", "Acme Corporation Pvt Ltd",
            Some("India"), None, Some("acme@example.com"), Some("+91 99999 99999"),
            Some("enterprise"), Some("high"), Some("active"),
            Some("{\"gst\":\"27AABCU9603R1ZX\",\"pan\":\"AABCU9603R\"}"),
            None,
            Some(&vec!["fintech".to_string()]),
            Some("Main client"),
        )
        .unwrap();

        let client = do_get_client(&conn, id).unwrap();
        assert_eq!(client.short_name, "Acme");
        assert_eq!(client.registered_business_name, "Acme Corporation Pvt Ltd");
        assert_eq!(client.country, Some("India".to_string()));
        assert_eq!(client.email, Some("acme@example.com".to_string()));
        assert_eq!(client.tags, vec!["fintech"]);
        assert_eq!(client.priority, Some("high".to_string()));
    }

    #[test]
    fn test_duplicate_short_name_rejected() {
        let conn = test_conn();
        do_create_client(&conn, "Acme", "Acme Corp", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let result = do_create_client(&conn, "Acme", "Acme Inc", None, None, None, None, None, None, None, None, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UNIQUE constraint"));
    }

    #[test]
    fn test_duplicate_registered_name_allowed() {
        let conn = test_conn();
        do_create_client(&conn, "Acme-US", "Acme LLC", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let result = do_create_client(&conn, "Acme-IN", "Acme LLC", None, None, None, None, None, None, None, None, None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_soft_delete_and_list() {
        let conn = test_conn();
        let id = do_create_client(&conn, "Acme", "Acme Corp", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let list = do_list_clients(&conn, None).unwrap();
        assert_eq!(list.len(), 1);

        do_delete_client(&conn, id).unwrap();
        let list_after = do_list_clients(&conn, None).unwrap();
        assert_eq!(list_after.len(), 0);

        let get_result = do_get_client(&conn, id);
        assert!(get_result.is_err());
    }

    #[test]
    fn test_search_clients() {
        let conn = test_conn();
        do_create_client(&conn, "Acme", "Acme Corp", None, None, Some("a@b.com"), None, None, None, None, None, None, None, None).unwrap();
        do_create_client(&conn, "Wayne", "Wayne Enterprises", None, None, None, None, None, None, None, None, None, None, None).unwrap();

        assert_eq!(do_list_clients(&conn, Some("Acme")).unwrap().len(), 1);
        assert_eq!(do_list_clients(&conn, Some("a@b")).unwrap().len(), 1);
        assert_eq!(do_list_clients(&conn, Some("zzzz")).unwrap().len(), 0);
    }

    #[test]
    fn test_dashboard_stats() {
        let conn = test_conn();
        let stats = do_get_dashboard_stats(&conn).unwrap();
        assert_eq!(stats.client_count, 0);
        assert_eq!(stats.finding_count, 0);
        assert_eq!(stats.project_count, 0);

        do_create_client(&conn, "Acme", "Acme Corp", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let stats = do_get_dashboard_stats(&conn).unwrap();
        assert_eq!(stats.client_count, 1);
    }

    #[test]
    fn test_update_client_with_history() {
        let conn = test_conn();
        let id = do_create_client(
            &conn, "Acme", "Acme Corp",
            Some("India"), None, Some("old@acme.com"), None, None, None, None, None, None, None, None
        ).unwrap();

        do_update_client(
            &conn, id,
            Some("Acme Inc"), None, Some("USA"), None, Some("new@acme.com"), None, None, Some("medium"), None, None, None, None, None
        ).unwrap();

        let client = do_get_client(&conn, id).unwrap();
        assert_eq!(client.short_name, "Acme Inc");
        assert_eq!(client.registered_business_name, "Acme Corp"); // unchanged
        assert_eq!(client.country, Some("USA".to_string()));
        assert_eq!(client.email, Some("new@acme.com".to_string()));
        assert_eq!(client.priority, Some("medium".to_string()));

        let history = do_get_client_history(&conn, id).unwrap();
        assert_eq!(history.len(), 4); // short_name, country, email, priority
        // Let me check: we updated short_name, country, email, priority = 4 changes
        let field_names: Vec<_> = history.iter().map(|h| h.field_name.clone()).collect();
        assert!(field_names.contains(&"short_name".to_string()));
        assert!(field_names.contains(&"country".to_string()));
        assert!(field_names.contains(&"email".to_string()));
        assert!(field_names.contains(&"priority".to_string()));
    }

    #[test]
    fn test_update_client_no_history_on_unchanged_field() {
        let conn = test_conn();
        let id = do_create_client(&conn, "Acme", "Acme Corp", None, None, None, None, None, None, None, None, None, None, None).unwrap();

        do_update_client(&conn, id, Some("Acme"), None, None, None, None, None, None, None, None, None, None, None, None
        ).unwrap();

        let history = do_get_client_history(&conn, id).unwrap();
        // short_name was set to same value, so no history should be written for it
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_client_mutations_write_audit_snapshots() {
        let conn = test_conn();
        let id = do_create_client(
            &conn, "Acme", "Acme Corp",
            None, None, Some("old@acme.com"), None, None, None, None, None, None, None, None
        ).unwrap();
        do_update_client(&conn, id, Some("Acme Inc"), None, None, Some("new@acme.com"), None, None, None, None, None, None, None, None, None
        ).unwrap();
        do_delete_client(&conn, id).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT action, old_value, new_value
                 FROM audit_log
                 WHERE table_name = 'clients'
                 ORDER BY id",
            )
            .unwrap();
        let rows: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "INSERT");
        assert!(rows[0].1.is_none());
        let created: serde_json::Value =
            serde_json::from_str(rows[0].2.as_deref().unwrap()).unwrap();
        assert_eq!(created["short_name"], "Acme");

        assert_eq!(rows[1].0, "UPDATE");
        let update_old: serde_json::Value =
            serde_json::from_str(rows[1].1.as_deref().unwrap()).unwrap();
        let update_new: serde_json::Value =
            serde_json::from_str(rows[1].2.as_deref().unwrap()).unwrap();
        assert_eq!(update_old["short_name"], "Acme");
        assert_eq!(update_new["short_name"], "Acme Inc");

        assert_eq!(rows[2].0, "DELETE");
        let delete_old: serde_json::Value =
            serde_json::from_str(rows[2].1.as_deref().unwrap()).unwrap();
        let delete_new: serde_json::Value =
            serde_json::from_str(rows[2].2.as_deref().unwrap()).unwrap();
        assert_eq!(delete_old["short_name"], "Acme Inc");
        assert_eq!(delete_new["is_active"], 0);
    }
}
