import { invoke } from "@tauri-apps/api/core";

export interface Attachment {
  id: number;
  entity_type: string;
  entity_id: number;
  filename: string;
  original_name: string;
  mime_type: string;
  file_size: number;
  sha256: string;
  sort_order: number;
  created_at: number;
}

export interface AttachmentInput {
  entity_type: string;
  entity_id: number;
  filename: string;
  original_name: string;
  mime_type: string;
  file_data_base64: string;
}

export async function uploadAttachment(
  input: AttachmentInput,
): Promise<Attachment> {
  return invoke<Attachment>("upload_attachment", { input });
}

export async function listAttachments(
  entityType: string,
  entityId: number,
): Promise<Attachment[]> {
  return invoke<Attachment[]>("list_attachments", {
    entity_type: entityType,
    entity_id: entityId,
  });
}

export async function deleteAttachment(id: number): Promise<void> {
  return invoke<void>("delete_attachment", { id });
}

export async function renameAttachment(
  id: number,
  newName: string,
): Promise<void> {
  return invoke<void>("rename_attachment", { id, new_name: newName });
}

export async function reorderAttachments(
  entityType: string,
  entityId: number,
  orderedIds: number[],
): Promise<void> {
  return invoke<void>("reorder_attachments", {
    entity_type: entityType,
    entity_id: entityId,
    ordered_ids: orderedIds,
  });
}

export async function readAttachmentFile(
  entityType: string,
  entityId: number,
  filename: string,
): Promise<string> {
  return invoke<string>("read_attachment_file", {
    entity_type: entityType,
    entity_id: entityId,
    filename,
  });
}

export async function getAttachmentThumbnail(
  entityType: string,
  entityId: number,
  filename: string,
): Promise<string> {
  return invoke<string>("get_attachment_thumbnail", {
    entity_type: entityType,
    entity_id: entityId,
    filename,
  });
}

export async function getTotalAttachmentStorage(): Promise<number> {
  return invoke<number>("get_total_attachment_storage");
}
