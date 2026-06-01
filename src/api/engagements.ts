import { invoke } from "@tauri-apps/api/core";

export type EngagementStatus =
  | "planned"
  | "scheduled"
  | "active"
  | "paused"
  | "completed";

export interface Engagement {
  id: number;
  client_id: number;
  client_name: string;
  name: string;
  target_area: string;
  assessment_kind: string;
  access_model: string;
  engagement_type: string;
  status: EngagementStatus;
  start_date: string | null;
  end_date: string | null;
  scope_summary: string | null;
  objectives: string[];
  notes: string | null;
  tags: string[];
  credentials_ready: boolean;
  payment_required: boolean;
  payment_cleared: boolean;
  budgeted_hours: number | null;
  created_at: number;
  updated_at: number;
}

export interface EngagementInput {
  client_id: number;
  name: string;
  target_area: string;
  assessment_kind: string;
  access_model: string;
  engagement_type: string;
  status: EngagementStatus;
  start_date?: string;
  end_date?: string;
  scope_summary?: string;
  objectives?: string[];
  notes?: string;
  tags?: string[];
  payment_required?: boolean;
  budgeted_hours?: number;
}

export async function createEngagement(
  input: EngagementInput,
): Promise<number> {
  return invoke<number>("create_engagement", { input });
}

export async function getEngagement(id: number): Promise<Engagement> {
  return invoke<Engagement>("get_engagement", { id });
}

export async function updateEngagement(
  id: number,
  input: EngagementInput,
): Promise<void> {
  return invoke<void>("update_engagement", { id, input });
}

export async function archiveEngagement(id: number): Promise<void> {
  return invoke<void>("archive_engagement", { id });
}

export async function listEngagements(options?: {
  clientId?: number;
  search?: string;
  status?: EngagementStatus;
}): Promise<Engagement[]> {
  return invoke<Engagement[]>("list_engagements", {
    clientId: options?.clientId,
    search: options?.search,
    status: options?.status,
  });
}

export async function transitionEngagementStatus(
  id: number,
  newStatus: EngagementStatus,
): Promise<void> {
  return invoke<void>("transition_engagement_status", {
    id,
    newStatus,
  });
}

export async function toggleEngagementGate(
  id: number,
  gate: "credentials_ready" | "payment_cleared",
  value: boolean,
): Promise<void> {
  return invoke<void>("toggle_engagement_gate", { id, gate, value });
}
