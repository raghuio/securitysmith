import { invoke } from "@tauri-apps/api/core";

export interface ScopeItem {
  id: number;
  engagement_id: number;
  item_type: string;
  value: string;
  is_in_scope: boolean;
  environment?: string;
  notes?: string;
  sort_order: number;
  created_at: number;
  updated_at: number;
}

export interface ScopeItemInput {
  engagement_id: number;
  item_type: string;
  value: string;
  is_in_scope?: boolean;
  environment?: string;
  notes?: string;
}

export async function listScopeItems(
  engagementId: number,
): Promise<ScopeItem[]> {
  return invoke<ScopeItem[]>("list_scope_items", {
    engagement_id: engagementId,
  });
}

export async function createScopeItem(input: ScopeItemInput): Promise<number> {
  return invoke<number>("create_scope_item", { input });
}

export async function updateScopeItem(
  id: number,
  input: ScopeItemInput,
): Promise<void> {
  return invoke<void>("update_scope_item", { id, input });
}

export async function deleteScopeItem(id: number): Promise<void> {
  return invoke<void>("delete_scope_item", { id });
}

export async function bulkImportScopeItems(
  engagementId: number,
  lines: string,
): Promise<number> {
  return invoke<number>("bulk_import_scope_items", {
    engagement_id: engagementId,
    lines,
  });
}

export async function exportScopeText(engagementId: number): Promise<string> {
  return invoke<string>("export_scope_text", { engagement_id: engagementId });
}
