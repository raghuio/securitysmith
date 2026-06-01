import { invoke } from "@tauri-apps/api/core";

export interface InvoiceItem {
  id: number;
  invoice_id: number;
  description: string;
  quantity: number;
  rate_cents: number;
  total_cents: number;
  created_at: number;
}

export interface Invoice {
  id: number;
  client_id: number;
  client_name: string;
  engagement_id: number | null;
  engagement_name: string | null;
  document_type: string;
  invoice_number: string;
  status: string;
  subtotal_cents: number;
  tax_rate_bps: number;
  discount_cents: number;
  discount_pct_bps: number;
  total_cents: number;
  currency: string;
  notes: string | null;
  items: InvoiceItem[];
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface InvoiceInput {
  client_id: number;
  engagement_id?: number;
  document_type: string;
  invoice_number: string;
  tax_rate_bps: number;
  discount_cents: number;
  discount_pct_bps: number;
  currency: string;
  notes?: string;
  items: { description: string; quantity: number; rate_cents: number }[];
}

export async function listInvoices(clientId?: number): Promise<Invoice[]> {
  return invoke<Invoice[]>("list_invoices", { clientId });
}

export async function getInvoice(id: number): Promise<Invoice> {
  return invoke<Invoice>("get_invoice", { id });
}

export async function createInvoice(input: InvoiceInput): Promise<number> {
  return invoke<number>("create_invoice", { input });
}

export async function updateInvoiceStatus(
  id: number,
  status: string,
): Promise<void> {
  return invoke("update_invoice_status", { id, status });
}

export async function archiveInvoice(id: number): Promise<void> {
  return invoke("archive_invoice", { id });
}

export async function addInvoiceItem(
  invoiceId: number,
  description: string,
  quantity: number,
  rateCents: number,
): Promise<number> {
  return invoke<number>("add_invoice_item", {
    invoiceId,
    description,
    quantity,
    rateCents,
  });
}

export async function updateInvoiceItem(
  id: number,
  updates: { description?: string; quantity?: number; rateCents?: number },
): Promise<void> {
  return invoke("update_invoice_item", { id, ...updates });
}

export async function deleteInvoiceItem(id: number): Promise<void> {
  return invoke("delete_invoice_item", { id });
}

export async function generateInvoicePdf(
  invoiceId: number,
  savePath: string,
): Promise<string> {
  return invoke<string>("generate_invoice_pdf", { invoiceId, savePath });
}
