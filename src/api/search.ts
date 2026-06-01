import { invoke } from "@tauri-apps/api/core";

export interface SearchResult {
  entity_type: string;
  entity_id: number;
  title: string;
  subtitle: string;
  relevance: number;
}

export async function globalSearch(
  query: string,
  limit?: number,
): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("global_search", { query, limit });
}

export async function rebuildSearchIndex(): Promise<void> {
  return invoke<void>("rebuild_search_index_command");
}
