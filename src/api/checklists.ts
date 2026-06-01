import { invoke } from "@tauri-apps/api/core";

export interface Checklist {
  id: number;
  name: string;
  description?: string;
  version?: string;
  is_builtin: boolean;
  is_active: boolean;
  created_at: number;
}

export interface ChecklistItem {
  id: number;
  checklist_id: number;
  category: string;
  test_id?: string;
  name: string;
  description?: string;
  sort_order: number;
}

export interface EngagementChecklistItem {
  id: number;
  engagement_id: number;
  checklist_item_id: number;
  status: string;
  linked_finding_id?: number;
  notes?: string;
  updated_at: number;
  checklist_item: ChecklistItem;
}

export interface ChecklistInput {
  name: string;
  description?: string;
  version?: string;
}

export interface ChecklistItemInput {
  checklist_id: number;
  category: string;
  test_id?: string;
  name: string;
  description?: string;
}

export async function listChecklists(): Promise<Checklist[]> {
  return invoke<Checklist[]>("list_checklists");
}

export async function getChecklistItems(
  checklistId: number,
): Promise<ChecklistItem[]> {
  return invoke<ChecklistItem[]>("get_checklist_items", {
    checklist_id: checklistId,
  });
}

export async function createChecklist(input: ChecklistInput): Promise<number> {
  return invoke<number>("create_checklist", { input });
}

export async function createChecklistItem(
  input: ChecklistItemInput,
): Promise<number> {
  return invoke<number>("create_checklist_item", { input });
}

export async function assignChecklistToEngagement(
  engagementId: number,
  checklistId: number,
): Promise<void> {
  return invoke<void>("assign_checklist_to_engagement", {
    engagement_id: engagementId,
    checklist_id: checklistId,
  });
}

export async function getEngagementChecklist(
  engagementId: number,
): Promise<EngagementChecklistItem[]> {
  return invoke<EngagementChecklistItem[]>("get_engagement_checklist", {
    engagement_id: engagementId,
  });
}

export async function updateEngagementChecklistItem(
  id: number,
  status: string,
  linkedFindingId?: number,
  notes?: string,
): Promise<void> {
  return invoke<void>("update_engagement_checklist_item", {
    id,
    status,
    linked_finding_id: linkedFindingId,
    notes,
  });
}

export async function getChecklistCoverage(
  engagementId: number,
): Promise<[number, number, number]> {
  return invoke<[number, number, number]>("get_checklist_coverage", {
    engagement_id: engagementId,
  });
}
