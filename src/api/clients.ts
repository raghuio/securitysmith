import { invoke } from "@tauri-apps/api/core";

export interface Client {
  id: number;
  name: string;
  contact_email: string | null;
  notes: string | null;
  tags: string[];
  tech_stack: string[];
  created_at: number;
  updated_at: number;
}

export interface DashboardStats {
  client_count: number;
  finding_count: number;
  engagement_count: number;
  findings_ready: boolean;
  engagements_ready: boolean;
}

export async function createClient(
  name: string,
  contact_email?: string,
  notes?: string,
  tags?: string[],
  tech_stack?: string[],
): Promise<number> {
  return invoke<number>("create_client", {
    name,
    contactEmail: contact_email ?? null,
    notes: notes ?? null,
    tags: tags ?? null,
    techStack: tech_stack ?? null,
  });
}

export async function getClient(id: number): Promise<Client> {
  return invoke<Client>("get_client", { id });
}

export async function updateClient(
  id: number,
  name?: string,
  contact_email?: string,
  notes?: string,
  tags?: string[],
  tech_stack?: string[],
): Promise<void> {
  return invoke<void>("update_client", {
    id,
    name: name ?? null,
    contactEmail: contact_email ?? null,
    notes: notes ?? null,
    tags: tags ?? null,
    techStack: tech_stack ?? null,
  });
}

export async function deleteClient(id: number): Promise<void> {
  return invoke<void>("delete_client", { id });
}

export async function listClients(search?: string): Promise<Client[]> {
  return invoke<Client[]>("list_clients", { search: search ?? null });
}

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke<DashboardStats>("get_dashboard_stats");
}
