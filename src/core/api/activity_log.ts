import { invoke } from "@tauri-apps/api/core";

export interface ActivityLogEntry {
  id: number;
  table_name: string;
  action: string;
  record_id: number;
  old_value: string;
  new_value: string;
  context: string;
  timestamp: number;
}

export interface ActivityLogFilters {
  table_name?: string;
  action?: string;
  search?: string;
  offset?: number;
  limit?: number;
}

export async function listActivityLog(
  filters: ActivityLogFilters,
): Promise<ActivityLogEntry[]> {
  return invoke<ActivityLogEntry[]>("list_activity_log", { filters });
}

export async function exportActivityLog(
  filters: ActivityLogFilters,
  filePath: string,
): Promise<number> {
  return invoke<number>("export_activity_log", { filters, filePath });
}

export async function getEntityHistory(
  tableName: string,
  entityId: number,
): Promise<ActivityLogEntry[]> {
  return invoke<ActivityLogEntry[]>("get_entity_history", {
    tableName,
    entityId,
  });
}

export async function pruneAuditLog(beforeDate: string): Promise<number> {
  return invoke<number>("prune_audit_log", { beforeDate });
}
