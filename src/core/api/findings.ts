import { invoke } from "@tauri-apps/api/core";

export interface AffectedEndpoint {
  method: string;
  path: string;
  description: string;
}

export interface Evidence {
  title: string;
  request: string;
  response: string;
}

export interface ImpactItem {
  title: string;
  explanation: string;
}

export interface RemediationItem {
  action: string;
  fix: string;
  code_snippet?: string;
}

export interface Reference {
  title: string;
  url: string;
}

export interface Finding {
  id: number;
  engagement_id: number;
  engagement_name: string;
  client_name: string;
  title: string;
  severity: Severity;
  cvss_score: number | null;
  owasp_category: string | null;
  cwe_id: string | null;
  overview: string;
  summary: string;
  affected_endpoints: AffectedEndpoint[];
  evidence: Evidence[];
  impact_items: ImpactItem[];
  remediation_items: RemediationItem[];
  steps_to_reproduce: string;
  references: Reference[];
  status: FindingStatus;
  tags: string[];
  notes: string | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface FindingPage {
  items: Finding[];
  total: number;
  offset: number;
  limit: number;
}

export interface FindingCounts {
  total: number;
  by_severity: Record<string, number>;
}

export type Severity = "critical" | "high" | "medium" | "low" | "informational";
export type FindingStatus =
  | "draft"
  | "confirmed"
  | "reported"
  | "fixed"
  | "accepted"
  | "false_positive"
  | "wont_fix";

export interface FindingInput {
  engagement_id: number;
  title: string;
  severity: Severity;
  overview: string;
  summary: string;
  affected_endpoints: AffectedEndpoint[];
  evidence: Evidence[];
  impact_items: ImpactItem[];
  remediation_items: RemediationItem[];
  steps_to_reproduce: string;
  cvss_score?: number;
  owasp_category?: string;
  cwe_id?: string;
  references?: Reference[];
  tags?: string[];
  notes?: string;
}

export async function createFinding(input: FindingInput): Promise<number> {
  return invoke<number>("create_finding", { input });
}

export async function getFinding(id: number): Promise<Finding> {
  return invoke<Finding>("get_finding", { id });
}

export async function updateFinding(
  id: number,
  input: FindingInput,
): Promise<void> {
  return invoke<void>("update_finding", { id, input });
}

export async function updateFindingStatus(
  id: number,
  status: FindingStatus,
): Promise<void> {
  return invoke<void>("update_finding_status", { id, status });
}

export async function duplicateFinding(id: number): Promise<number> {
  return invoke<number>("duplicate_finding", { id });
}

export async function archiveFinding(id: number): Promise<void> {
  return invoke<void>("archive_finding", { id });
}

export async function listFindings(options?: {
  engagementId?: number;
  clientId?: number;
  search?: string;
  severity?: Severity;
  status?: FindingStatus;
  owaspCategory?: string;
  offset?: number;
  limit?: number;
}): Promise<FindingPage> {
  return invoke<FindingPage>("list_findings", {
    engagementId: options?.engagementId,
    clientId: options?.clientId,
    search: options?.search,
    severity: options?.severity,
    status: options?.status,
    owaspCategory: options?.owaspCategory,
    offset: options?.offset ?? 0,
    limit: options?.limit ?? 50,
  });
}

export async function getFindingCounts(): Promise<FindingCounts> {
  return invoke<FindingCounts>("get_finding_counts");
}
