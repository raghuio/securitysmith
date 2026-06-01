import { invoke } from "@tauri-apps/api/core";

export interface Report {
  id: number;
  engagement_id: number;
  engagement_name: string;
  client_name: string;
  name: string;
  executive_summary: string;
  appendix: string;
  included_finding_ids: number[];
  status: string;
  generated_at: number | null;
  file_path: string | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export async function listReports(engagementId?: number): Promise<Report[]> {
  return invoke<Report[]>("list_reports", {
    engagementId: engagementId || null,
  });
}

export async function getReport(id: number): Promise<Report> {
  return invoke<Report>("get_report", { id });
}

export async function createReport(
  engagementId: number,
  name: string,
  executiveSummary: string,
  appendix: string,
  includedFindingIds: number[],
): Promise<number> {
  return invoke<number>("create_report", {
    engagementId,
    name,
    executiveSummary,
    appendix,
    includedFindingIds,
  });
}

export async function updateReport(
  id: number,
  updates: Partial<
    Pick<
      Report,
      "name" | "executive_summary" | "appendix" | "included_finding_ids"
    >
  >,
): Promise<void> {
  return invoke<void>("update_report", {
    id,
    name: updates.name || null,
    executiveSummary: updates.executive_summary || null,
    appendix: updates.appendix || null,
    includedFindingIds: updates.included_finding_ids || null,
  });
}

export async function archiveReport(id: number): Promise<void> {
  return invoke<void>("archive_report", { id });
}

export async function generateReportPdf(id: number): Promise<string> {
  return invoke<string>("generate_report_pdf", { id });
}
