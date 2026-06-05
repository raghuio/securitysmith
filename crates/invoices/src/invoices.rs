use securitysmith_core::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvoiceItem {
    pub id: u32,
    pub invoice_id: u32,
    pub description: String,
    pub quantity: u32,
    pub rate_cents: u32,
    pub total_cents: u32,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct Invoice {
    pub id: u32,
    pub client_id: u32,
    pub client_name: String,
    pub engagement_id: Option<u32>,
    pub engagement_name: Option<String>,
    pub document_type: String,
    pub invoice_number: String,
    pub status: String,
    pub subtotal_cents: u32,
    pub tax_rate_bps: u32,
    pub discount_cents: u32,
    pub discount_pct_bps: u32,
    pub total_cents: u32,
    pub currency: String,
    pub notes: Option<String>,
    pub items: Vec<InvoiceItem>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct InvoiceItemInput {
    pub description: String,
    pub quantity: u32,
    pub rate_cents: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct InvoiceInput {
    pub client_id: u32,
    pub engagement_id: Option<u32>,
    pub document_type: String,
    pub invoice_number: String,
    pub tax_rate_bps: u32,
    pub discount_cents: u32,
    pub discount_pct_bps: u32,
    pub currency: String,
    pub notes: Option<String>,
    pub items: Vec<InvoiceItemInput>,
}

fn recalc_total(
    items: &[InvoiceItemInput],
    tax_rate_bps: u32,
    discount_cents: u32,
    discount_pct_bps: u32,
) -> (u32, u32) {
    let subtotal: u64 = items
        .iter()
        .map(|i| i.quantity as u64 * i.rate_cents as u64)
        .sum();
    let pct_discount = (subtotal * discount_pct_bps as u64) / 10000u64;
    let total_after_discount = subtotal
        .saturating_sub(discount_cents as u64)
        .saturating_sub(pct_discount);
    let tax = (total_after_discount * tax_rate_bps as u64) / 10000u64;
    let total = total_after_discount + tax;
    (subtotal as u32, total as u32)
}

fn do_get_invoice(conn: &Connection, id: u32) -> Result<Invoice, String> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.client_id, c.name, i.engagement_id, e.name, i.document_type, i.invoice_number, i.status, i.subtotal_cents, i.tax_rate_bps, i.discount_cents, i.discount_pct_bps, i.total_cents, i.currency, i.notes, i.is_active, i.created_at, i.updated_at
         FROM invoices i JOIN clients c ON i.client_id = c.id LEFT JOIN engagements e ON i.engagement_id = e.id
         WHERE i.id = ? AND i.is_active = 1"
    ).map_err(|e| format!("Prepare failed: {e}"))?;
    let invoice: Option<Invoice> = stmt
        .query_map(params![id], |row| {
            Ok(Invoice {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                engagement_id: row.get(3)?,
                engagement_name: row.get(4)?,
                document_type: row.get(5)?,
                invoice_number: row.get(6)?,
                status: row.get(7)?,
                subtotal_cents: row.get(8)?,
                tax_rate_bps: row.get(9)?,
                discount_cents: row.get(10)?,
                discount_pct_bps: row.get(11)?,
                total_cents: row.get(12)?,
                currency: row.get(13)?,
                notes: row.get(14)?,
                items: Vec::new(),
                is_active: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .next()
        .transpose()
        .map_err(|e| format!("Row failed: {e}"))?;

    let inv = invoice.ok_or_else(|| "Invoice not found.".to_string())?;
    let mut stmt = conn.prepare("SELECT id, invoice_id, description, quantity, rate_cents, total_cents, created_at FROM invoice_items WHERE invoice_id = ?").map_err(|e| format!("Prepare failed: {e}"))?;
    let items: Vec<InvoiceItem> = stmt
        .query_map(params![id], |row| {
            Ok(InvoiceItem {
                id: row.get(0)?,
                invoice_id: row.get(1)?,
                description: row.get(2)?,
                quantity: row.get(3)?,
                rate_cents: row.get(4)?,
                total_cents: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    // manual total recalc
    let item_inputs: Vec<InvoiceItemInput> = items
        .iter()
        .map(|i| InvoiceItemInput {
            description: i.description.clone(),
            quantity: i.quantity,
            rate_cents: i.rate_cents,
        })
        .collect();
    let (subtotal, total) = recalc_total(
        &item_inputs,
        inv.tax_rate_bps,
        inv.discount_cents,
        inv.discount_pct_bps,
    );
    let mut inv = inv;
    inv.items = items;
    inv.subtotal_cents = subtotal;
    inv.total_cents = total;
    Ok(inv)
}

fn do_create_invoice(conn: &Connection, input: &InvoiceInput) -> Result<u32, String> {
    let (subtotal, total) = recalc_total(
        &input.items,
        input.tax_rate_bps,
        input.discount_cents,
        input.discount_pct_bps,
    );
    conn.execute(
        "INSERT INTO invoices (client_id, engagement_id, document_type, invoice_number, status, subtotal_cents, tax_rate_bps, discount_cents, discount_pct_bps, total_cents, currency, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%s', 'now'), strftime('%s', 'now'))",
        params![input.client_id, input.engagement_id, input.document_type, input.invoice_number, subtotal, input.tax_rate_bps, input.discount_cents, input.discount_pct_bps, total, input.currency, input.notes],
    ).map_err(|e| format!("Failed to create invoice: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    for item in &input.items {
        let item_total = item.quantity * item.rate_cents;
        conn.execute(
            "INSERT INTO invoice_items (invoice_id, description, quantity, rate_cents, total_cents, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s', 'now'))",
            params![id, item.description, item.quantity, item.rate_cents, item_total],
        ).map_err(|e| format!("Failed to create item: {e}"))?;
    }
    let new = do_get_invoice(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["invoices", "CREATE", id, "", new_json],
    ).map_err(|e| format!("Audit failed: {e}"))?;
    Ok(id)
}

fn do_update_invoice_status(conn: &Connection, id: u32, status: &str) -> Result<(), String> {
    if !["draft", "sent", "paid", "cancelled", "overdue"].contains(&status) {
        return Err("Invalid invoice status.".to_string());
    }
    let old = do_get_invoice(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "UPDATE invoices SET status = ?1, updated_at = strftime('%s', 'now') WHERE id = ?2",
        params![status, id],
    )
    .map_err(|e| format!("Update failed: {e}"))?;
    let new = do_get_invoice(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["invoices", "STATUS_CHANGE", id, old_json, new_json],
    ).map_err(|e| format!("Audit failed: {e}"))?;

    // update engagement payment gate
    if let Some(eid) = old.engagement_id {
        let payment_cleared = if status == "paid" { 1 } else { 0 };
        conn.execute(
            "UPDATE engagements SET payment_cleared = ? WHERE id = ?",
            params![payment_cleared, eid],
        )
        .map_err(|e| format!("Gate update failed: {e}"))?;
    }
    Ok(())
}

fn do_archive_invoice(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_invoice(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "UPDATE invoices SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Archive failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["invoices", "ARCHIVE", id, old_json, ""],
    ).map_err(|e| format!("Audit failed: {e}"))?;
    Ok(())
}

fn do_list_invoices(conn: &Connection, client_id: Option<u32>) -> Result<Vec<Invoice>, String> {
    let mut sql = "SELECT i.id, i.client_id, c.name, i.engagement_id, e.name, i.document_type, i.invoice_number, i.status, i.subtotal_cents, i.tax_rate_bps, i.discount_cents, i.discount_pct_bps, i.total_cents, i.currency, i.notes, i.is_active, i.created_at, i.updated_at
     FROM invoices i JOIN clients c ON i.client_id = c.id LEFT JOIN engagements e ON i.engagement_id = e.id
     WHERE i.is_active = 1".to_string();
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(cid) = client_id {
        sql.push_str(" AND i.client_id = ?");
        ps.push(Box::new(cid));
    }
    sql.push_str(" ORDER BY i.updated_at DESC");
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let mut invoices: Vec<Invoice> = stmt
        .query_map(rusqlite::params_from_iter(p_refs), |row| {
            Ok(Invoice {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                engagement_id: row.get(3)?,
                engagement_name: row.get(4)?,
                document_type: row.get(5)?,
                invoice_number: row.get(6)?,
                status: row.get(7)?,
                subtotal_cents: row.get(8)?,
                tax_rate_bps: row.get(9)?,
                discount_cents: row.get(10)?,
                discount_pct_bps: row.get(11)?,
                total_cents: row.get(12)?,
                currency: row.get(13)?,
                notes: row.get(14)?,
                items: Vec::new(),
                is_active: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    for inv in &mut invoices {
        let mut stmt = conn.prepare("SELECT id, invoice_id, description, quantity, rate_cents, total_cents, created_at FROM invoice_items WHERE invoice_id = ?").map_err(|e| format!("Prepare items: {e}"))?;
        let items: Vec<InvoiceItem> = stmt
            .query_map(params![inv.id], |row| {
                Ok(InvoiceItem {
                    id: row.get(0)?,
                    invoice_id: row.get(1)?,
                    description: row.get(2)?,
                    quantity: row.get(3)?,
                    rate_cents: row.get(4)?,
                    total_cents: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Items query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        inv.items = items;
    }
    Ok(invoices)
}

#[tauri::command]
pub fn list_invoices(
    state: State<AppState>,
    client_id: Option<u32>,
) -> Result<Vec<Invoice>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_invoices(conn, client_id)
}

#[tauri::command]
pub fn get_invoice(state: State<AppState>, id: u32) -> Result<Invoice, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_get_invoice(conn, id)
}

#[tauri::command]
pub fn create_invoice(state: State<AppState>, input: InvoiceInput) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_create_invoice(conn, &input)
}

#[tauri::command]
pub fn update_invoice_status(
    state: State<AppState>,
    id: u32,
    status: String,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_update_invoice_status(conn, id, &status)
}

#[tauri::command]
pub fn archive_invoice(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_archive_invoice(conn, id)
}

fn recalc_and_update_invoice_totals(conn: &Connection, invoice_id: u32) -> Result<(), String> {
    let mut stmt = conn
        .prepare("SELECT quantity, rate_cents FROM invoice_items WHERE invoice_id = ?")
        .map_err(|e| format!("Prepare items: {e}"))?;
    let items: Vec<(u32, u32)> = stmt
        .query_map(params![invoice_id], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|e| format!("Query items: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    let subtotal: u64 = items.iter().map(|(q, r)| (*q as u64) * (*r as u64)).sum();

    let (tax_rate_bps, discount_cents, discount_pct_bps): (u32, u32, u32) = conn
        .query_row(
            "SELECT tax_rate_bps, discount_cents, discount_pct_bps FROM invoices WHERE id = ?",
            [invoice_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("Get invoice rates: {e}"))?;

    let pct_discount = (subtotal * discount_pct_bps as u64) / 10000u64;
    let total_after_discount = subtotal
        .saturating_sub(discount_cents as u64)
        .saturating_sub(pct_discount);
    let tax = (total_after_discount * tax_rate_bps as u64) / 10000u64;
    let total = total_after_discount + tax;

    conn.execute(
        "UPDATE invoices SET subtotal_cents = ?, total_cents = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![subtotal as u32, total as u32, invoice_id],
    )
    .map_err(|e| format!("Update totals: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn add_invoice_item(
    state: State<AppState>,
    invoice_id: u32,
    description: String,
    quantity: u32,
    rate_cents: u32,
) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    let total = quantity * rate_cents;
    conn.execute(
        "INSERT INTO invoice_items (invoice_id, description, quantity, rate_cents, total_cents, created_at) VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s', 'now'))",
        params![invoice_id, description, quantity, rate_cents, total],
    )
    .map_err(|e| format!("Insert item: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    recalc_and_update_invoice_totals(conn, invoice_id)?;
    Ok(id)
}

#[tauri::command]
pub fn update_invoice_item(
    state: State<AppState>,
    id: u32,
    description: Option<String>,
    quantity: Option<u32>,
    rate_cents: Option<u32>,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    let invoice_id: u32 = conn
        .query_row(
            "SELECT invoice_id FROM invoice_items WHERE id = ?",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| "Item not found.".to_string())?;
    let mut updates: Vec<(&str, Box<dyn rusqlite::ToSql>)> = Vec::new();
    if let Some(d) = description {
        updates.push(("description = ?", Box::new(d)));
    }
    if let Some(q) = quantity {
        updates.push(("quantity = ?", Box::new(q)));
    }
    if let Some(r) = rate_cents {
        updates.push(("rate_cents = ?", Box::new(r)));
    }
    if updates.is_empty() {
        return Ok(());
    }
    updates.push(("total_cents = quantity * rate_cents", Box::new(1)));
    let set_clause = updates
        .iter()
        .map(|(c, _)| *c)
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("UPDATE invoice_items SET {set_clause} WHERE id = ?");
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = updates.into_iter().map(|(_, v)| v).collect();
    ps.push(Box::new(id));
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, rusqlite::params_from_iter(p_refs))
        .map_err(|e| format!("Update item: {e}"))?;
    recalc_and_update_invoice_totals(conn, invoice_id)?;
    Ok(())
}

#[tauri::command]
pub fn delete_invoice_item(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    let invoice_id: u32 = conn
        .query_row(
            "SELECT invoice_id FROM invoice_items WHERE id = ?",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| "Item not found.".to_string())?;
    conn.execute("DELETE FROM invoice_items WHERE id = ?", params![id])
        .map_err(|e| format!("Delete item: {e}"))?;
    recalc_and_update_invoice_totals(conn, invoice_id)?;
    Ok(())
}

use printpdf::{BuiltinFont, Mm, PdfDocument};
use std::io::BufWriter;

#[tauri::command]
pub fn generate_invoice_pdf(
    state: State<AppState>,
    invoice_id: u32,
    save_path: String,
) -> Result<String, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;

    let invoice = do_get_invoice(conn, invoice_id)?;

    let mut items = invoice.items.clone();
    items.sort_by_key(|i| i.id);

    let (doc, page1, layer1) =
        PdfDocument::new(&invoice.invoice_number, Mm(210.0), Mm(297.0), "Page1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Font: {e}"))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Font: {e}"))?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    let mut y = 270.0;
    current_layer.use_text(
        format!("{} — {}", invoice.client_name, invoice.invoice_number),
        18.0,
        Mm(20.0),
        Mm(y),
        &font_bold,
    );
    y -= 10.0;
    current_layer.use_text(
        format!(
            "Status: {} | Currency: {}",
            invoice.status, invoice.currency
        ),
        11.0,
        Mm(20.0),
        Mm(y),
        &font,
    );
    y -= 10.0;
    if let Some(ref notes) = invoice.notes {
        current_layer.use_text(format!("Notes: {notes}"), 10.0, Mm(20.0), Mm(y), &font);
        y -= 6.0;
    }

    y -= 10.0;
    current_layer.use_text("Line Items", 14.0, Mm(20.0), Mm(y), &font_bold);
    y -= 8.0;
    for item in &items {
        let line = format!(
            "{} x {} @ {:.2} = {:.2}",
            item.description,
            item.quantity,
            (item.rate_cents as f64) / 100.0,
            (item.total_cents as f64) / 100.0
        );
        current_layer.use_text(line, 10.0, Mm(20.0), Mm(y), &font);
        y -= 6.0;
    }

    y -= 8.0;
    current_layer.use_text(
        format!(
            "Subtotal: {:.2} | Tax: {} bps | Discount: {} | Total: {:.2}",
            (invoice.subtotal_cents as f64) / 100.0,
            invoice.tax_rate_bps,
            if invoice.discount_cents > 0 {
                format!("{:.2}", (invoice.discount_cents as f64) / 100.0)
            } else {
                format!("{}%", (invoice.discount_pct_bps as f64) / 100.0)
            },
            (invoice.total_cents as f64) / 100.0,
        ),
        11.0,
        Mm(20.0),
        Mm(y),
        &font_bold,
    );

    doc.save(&mut BufWriter::new(
        std::fs::File::create(&save_path).map_err(|e| format!("File: {e}"))?,
    ))
    .map_err(|e| format!("PDF save: {e}"))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["invoices", "PDF_GENERATED", invoice_id, "", "", format!("path={save_path}")],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;

    Ok(save_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use securitysmith_core::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn make_client(conn: &Connection) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![format!("Client-{n}")],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    fn make_invoice_input(client_id: u32) -> InvoiceInput {
        InvoiceInput {
            client_id,
            engagement_id: None,
            document_type: "invoice".to_string(),
            invoice_number: format!(
                "INV-{}",
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            ),
            tax_rate_bps: 0,
            discount_cents: 0,
            discount_pct_bps: 0,
            currency: "USD".to_string(),
            notes: None,
            items: vec![],
        }
    }

    #[test]
    fn test_invoice_crud() {
        let conn = test_conn();
        let cid = make_client(&conn);
        let id = do_create_invoice(&conn, &make_invoice_input(cid)).unwrap();
        let inv = do_get_invoice(&conn, id).unwrap();
        assert_eq!(inv.client_id, cid);
        assert_eq!(inv.invoice_number.contains("INV-"), true);
    }

    #[test]
    fn test_invoice_status_transitions() {
        let conn = test_conn();
        let cid = make_client(&conn);
        let id = do_create_invoice(&conn, &make_invoice_input(cid)).unwrap();
        do_update_invoice_status(&conn, id, "sent").unwrap();
        let inv = do_get_invoice(&conn, id).unwrap();
        assert_eq!(inv.status, "sent");
        do_update_invoice_status(&conn, id, "paid").unwrap();
        let inv = do_get_invoice(&conn, id).unwrap();
        assert_eq!(inv.status, "paid");
    }

    #[test]
    fn test_invoice_archive() {
        let conn = test_conn();
        let cid = make_client(&conn);
        let id = do_create_invoice(&conn, &make_invoice_input(cid)).unwrap();
        do_archive_invoice(&conn, id).unwrap();
        let list = do_list_invoices(&conn, None).unwrap();
        assert!(!list.iter().any(|i| i.id == id));
    }

    #[test]
    fn test_invoice_invalid_status_rejected() {
        let conn = test_conn();
        let cid = make_client(&conn);
        let id = do_create_invoice(&conn, &make_invoice_input(cid)).unwrap();
        let err = do_update_invoice_status(&conn, id, "bogus").unwrap_err();
        assert!(err.to_lowercase().contains("invalid") || err.to_lowercase().contains("status"));
    }
}
