import { invoke } from "@tauri-apps/api/core";

export interface DataPoint {
  label: string;
  value: number;
}

export interface TimeSeriesPoint {
  period: string;
  critical: number;
  high: number;
  medium: number;
  low: number;
  informational: number;
}

export interface RemediationRate {
  total: number;
  fixed_on_time: number;
  overdue: number;
  rate_percent: number;
}

export interface TimelineEntry {
  engagement_id: number;
  name: string;
  client_name: string;
  start_date?: string;
  end_date?: string;
  status: string;
}

export interface BudgetComparison {
  engagement_id: number;
  name: string;
  budgeted: number;
  actual: number;
}

export async function getSeverityDistribution(
  dateFrom?: string,
  dateTo?: string,
): Promise<DataPoint[]> {
  return invoke<DataPoint[]>("get_severity_distribution", {
    date_from: dateFrom,
    date_to: dateTo,
  });
}

export async function getTopCategories(limit: number): Promise<DataPoint[]> {
  return invoke<DataPoint[]>("get_top_categories", { limit });
}

export async function getFindingsOverTime(
  interval: string,
): Promise<TimeSeriesPoint[]> {
  return invoke<TimeSeriesPoint[]>("get_findings_over_time", { interval });
}

export async function getRemediationRate(): Promise<RemediationRate> {
  return invoke<RemediationRate>("get_remediation_rate");
}

export async function getRevenueByClient(): Promise<DataPoint[]> {
  return invoke<DataPoint[]>("get_revenue_by_client");
}

export async function getEngagementTimeline(): Promise<TimelineEntry[]> {
  return invoke<TimelineEntry[]>("get_engagement_timeline");
}

export async function getTimeByActivity(): Promise<DataPoint[]> {
  return invoke<DataPoint[]>("get_time_by_activity");
}

export async function getBudgetVsActual(): Promise<BudgetComparison[]> {
  return invoke<BudgetComparison[]>("get_budget_vs_actual");
}
