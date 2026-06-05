import { invoke } from "@tauri-apps/api/core";

export interface TimeEntry {
  id: number;
  engagement_id: number;
  entry_date: string;
  hours: number;
  description?: string;
  activity_type: string;
  is_billable: boolean;
  created_at: number;
  updated_at: number;
}

export interface TimeEntryInput {
  engagement_id: number;
  entry_date: string;
  hours: number;
  description?: string;
  activity_type: string;
  is_billable?: boolean;
}

export interface WeeklySummary {
  engagement_id: number;
  engagement_name: string;
  total_hours: number;
  billable_hours: number;
}

export interface BudgetStatus {
  engagement_id: number;
  engagement_name: string;
  budgeted_hours: number;
  logged_hours: number;
  percentage: number;
}

export async function listTimeEntries(
  engagementId: number,
): Promise<TimeEntry[]> {
  return invoke<TimeEntry[]>("list_time_entries", {
    engagement_id: engagementId,
  });
}

export async function createTimeEntry(input: TimeEntryInput): Promise<number> {
  return invoke<number>("create_time_entry", { input });
}

export async function updateTimeEntry(
  id: number,
  input: TimeEntryInput,
): Promise<void> {
  return invoke<void>("update_time_entry", { id, input });
}

export async function deleteTimeEntry(id: number): Promise<void> {
  return invoke<void>("delete_time_entry", { id });
}

export async function getWeeklySummary(
  dateFrom: string,
  dateTo: string,
): Promise<WeeklySummary[]> {
  return invoke<WeeklySummary[]>("get_weekly_summary", {
    date_from: dateFrom,
    date_to: dateTo,
  });
}

export async function getBudgetStatus(): Promise<BudgetStatus[]> {
  return invoke<BudgetStatus[]>("get_budget_status");
}

export async function createInvoiceFromTime(
  engagementId: number,
  dateFrom: string,
  dateTo: string,
  rate: number,
): Promise<number> {
  return invoke<number>("create_invoice_from_time", {
    engagement_id: engagementId,
    date_from: dateFrom,
    date_to: dateTo,
    rate,
  });
}
