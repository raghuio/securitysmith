import { invoke } from "@tauri-apps/api/core";

export type CredentialStatus =
  | "not_verified"
  | "working"
  | "not_working"
  | "expired";

export interface Credential {
  id: number;
  engagement_id: number;
  label: string;
  credential_type: string;
  value: string;
  notes: string | null;
  status: CredentialStatus;
  created_at: number;
  updated_at: number;
}

export interface CredentialInput {
  engagement_id: number;
  label: string;
  credential_type: string;
  value: string;
  notes?: string;
}

export interface CredentialUpdate {
  label?: string;
  credential_type?: string;
  value?: string;
  notes?: string;
  status?: CredentialStatus;
}

export async function createCredential(
  input: CredentialInput,
): Promise<number> {
  return invoke<number>("create_credential", { input });
}

export async function getCredential(id: number): Promise<Credential> {
  return invoke<Credential>("get_credential", { id });
}

export async function updateCredential(
  id: number,
  update: CredentialUpdate,
): Promise<void> {
  return invoke<void>("update_credential", { id, update });
}

export async function deleteCredential(id: number): Promise<void> {
  return invoke<void>("delete_credential", { id });
}

export async function listCredentials(
  engagementId: number,
): Promise<Credential[]> {
  return invoke<Credential[]>("list_credentials", { engagementId });
}
