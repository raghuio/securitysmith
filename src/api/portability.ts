import { invoke } from "@tauri-apps/api/core";

export interface ExportTreeClient {
  id: number;
  name: string;
  engagements: ExportTreeEngagement[];
  documents: ExportTreeDocument[];
  invoices: ExportTreeInvoice[];
}

export interface ExportTreeEngagement {
  id: number;
  name: string;
  finding_count: number;
  credential_count: number;
  document_count: number;
  finding_ids: number[];
  credential_ids: number[];
  document_ids: number[];
}

export interface ExportTreeDocument {
  id: number;
  name: string;
  document_type: string;
}

export interface ExportTreeInvoice {
  id: number;
  invoice_number: string;
}

export interface ExportTreeTemplate {
  id: number;
  name: string;
  category: string;
}

export interface ExportTree {
  clients: ExportTreeClient[];
  templates: ExportTreeTemplate[];
}

export interface ExportSelection {
  client_ids: number[];
  engagement_ids: number[];
  finding_ids: number[];
  credential_ids: number[];
  document_ids: number[];
  invoice_ids: number[];
  template_ids: number[];
}

export interface ExportResult {
  file_path: string;
  entity_counts: Record<string, number>;
}

export interface ImportConflict {
  entity_type: string;
  import_name: string;
}

export interface ImportPreview {
  compatible: boolean;
  conflicts: ImportConflict[];
}

export interface ConflictResolution {
  reference_key: string;
  action: "skip" | "overwrite" | "rename";
}

export interface ImportResult {
  imported: Record<string, number>;
  skipped: Record<string, number>;
}

export async function getExportTree(): Promise<ExportTree> {
  return invoke<ExportTree>("get_export_tree");
}

export async function createExport(
  selection: ExportSelection,
  includeCredentialValues: boolean,
  savePath: string,
): Promise<ExportResult> {
  return invoke<ExportResult>("create_export", {
    selection,
    includeCredentialValues,
    savePath,
  });
}

export async function createEncryptedExport(
  selection: ExportSelection,
  includeCredentialValues: boolean,
  savePath: string,
  password: string,
): Promise<ExportResult> {
  return invoke<ExportResult>("create_encrypted_export", {
    selection,
    includeCredentialValues,
    savePath,
    password,
  });
}

export async function isImportEncrypted(filePath: string): Promise<boolean> {
  return invoke<boolean>("is_import_encrypted", { filePath });
}

export async function decryptImportToTemp(
  filePath: string,
  password: string,
): Promise<string> {
  return invoke<string>("decrypt_import_to_temp", { filePath, password });
}

export async function previewImport(filePath: string): Promise<ImportPreview> {
  return invoke<ImportPreview>("preview_import", { filePath });
}

export async function executeImport(
  filePath: string,
  conflictResolutions: ConflictResolution[],
): Promise<ImportResult> {
  return invoke<ImportResult>("execute_import", {
    filePath,
    conflictResolutions,
  });
}

export async function exportVaultJson(
  filePath: string,
): Promise<Record<string, number> & { file_path: string }> {
  return invoke("export_vault_json", { filePath });
}
