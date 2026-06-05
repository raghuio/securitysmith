import { invoke } from "@tauri-apps/api/core";

export interface RetestEngagement {
  id: number;
  original_engagement_id: number;
  original_name: string;
  name: string;
  client_name: string;
  status: string;
  created_at: number;
}

export async function createRetestEngagement(
  originalEngagementId: number,
): Promise<number> {
  return invoke<number>("create_retest_engagement", {
    original_engagement_id: originalEngagementId,
  });
}

export async function listRetestEngagements(
  originalEngagementId: number,
): Promise<RetestEngagement[]> {
  return invoke<RetestEngagement[]>("list_retest_engagements", {
    original_engagement_id: originalEngagementId,
  });
}

export async function getRetestComparison(
  retestEngagementId: number,
): Promise<Record<string, unknown>[]> {
  return invoke<Record<string, unknown>[]>("get_retest_comparison", {
    retest_engagement_id: retestEngagementId,
  });
}

export async function bulkUpdateFindingStatus(
  findingIds: number[],
  clientResponse: string,
): Promise<void> {
  return invoke<void>("bulk_update_finding_status", {
    finding_ids: findingIds,
    client_response: clientResponse,
  });
}

export async function getOverdueFindings(): Promise<Record<string, unknown>[]> {
  return invoke<Record<string, unknown>[]>("get_overdue_findings");
}
