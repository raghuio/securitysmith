import { invoke } from "@tauri-apps/api/core";

export interface Contact {
  id: number;
  client_id: number;
  name: string;
  email: string;
  phone?: string;
  role: string;
  role_label?: string;
  title?: string;
  notes?: string;
  is_primary: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface ContactInput {
  client_id: number;
  name: string;
  email: string;
  phone?: string;
  role: string;
  role_label?: string;
  title?: string;
  notes?: string;
  is_primary?: boolean;
}

export async function listContacts(clientId: number): Promise<Contact[]> {
  return invoke<Contact[]>("list_contacts", { client_id: clientId });
}

export async function getContact(id: number): Promise<Contact> {
  return invoke<Contact>("get_contact", { id });
}

export async function createContact(input: ContactInput): Promise<number> {
  return invoke<number>("create_contact", { input });
}

export async function updateContact(
  id: number,
  input: ContactInput,
): Promise<void> {
  return invoke<void>("update_contact", { id, input });
}

export async function deleteContact(id: number): Promise<void> {
  return invoke<void>("delete_contact", { id });
}
