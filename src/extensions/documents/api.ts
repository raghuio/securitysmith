import { invoke } from "@tauri-apps/api/core";

export interface Document {
  id: number;
  client_id: number;
  client_name: string;
  engagement_id: number | null;
  engagement_name: string | null;
  name: string;
  document_type: string;
  content: string;
  status: string;
  template_id: number | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export async function listDocuments(
  clientId?: number,
  engagementId?: number,
): Promise<Document[]> {
  return invoke<Document[]>("list_documents", { clientId, engagementId });
}

export async function getDocument(id: number): Promise<Document> {
  return invoke<Document>("get_document", { id });
}

export async function createDocument(
  clientId: number,
  name: string,
  documentType: string,
  content: string,
  engagementId?: number,
  templateId?: number,
): Promise<number> {
  return invoke<number>("create_document", {
    clientId,
    engagementId,
    name,
    documentType,
    content,
    templateId,
  });
}

export async function updateDocument(
  id: number,
  updates: { name?: string; content?: string; status?: string },
): Promise<void> {
  return invoke("update_document", { id, ...updates });
}

export async function archiveDocument(id: number): Promise<void> {
  return invoke("archive_document", { id });
}

export async function renderDocumentPlaceholders(
  content: string,
  clientId: number,
  engagementId?: number,
): Promise<string> {
  return invoke<string>("render_document_placeholders", {
    content,
    clientId,
    engagementId,
  });
}
