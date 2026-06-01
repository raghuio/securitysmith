import { invoke } from "@tauri-apps/api/core";

export interface ParsedFinding {
  title: string;
  severity: string;
  overview: string;
  summary: string;
  affected_endpoints: { method: string; path: string; description: string }[];
  remediation_items: {
    action: string;
    fix: string;
    code_snippet?: string | null;
  }[];
  references: { title: string; url: string }[];
  source_tool: string;
  source_id: string | null;
  is_duplicate: boolean;
  duplicate_of: number | null;
}

export interface ImportPreview {
  findings: ParsedFinding[];
  total_parsed: number;
  duplicates_found: number;
  format: string;
}

export interface ImportResult {
  imported_count: number;
  skipped_count: number;
}

export type ImportFormat =
  | "nessus"
  | "burp"
  | "zap_json"
  | "nmap"
  | "nuclei"
  | "csv";

export interface CsvColumnMapping {
  title: number;
  severity: number;
  description?: number;
  remediation?: number;
  affected_url?: number;
}

export async function parseImportFile(
  filePath: string,
  format: ImportFormat,
  engagementId: number,
  csvMapping?: CsvColumnMapping,
): Promise<ImportPreview> {
  return invoke<ImportPreview>("parse_import_file", {
    filePath,
    format,
    engagementId,
    csvMapping: csvMapping || null,
  });
}

export async function commitImport(
  engagementId: number,
  findings: ParsedFinding[],
): Promise<ImportResult> {
  return invoke<ImportResult>("commit_import", {
    engagementId,
    findings,
  });
}
