import { invoke } from "@tauri-apps/api/core";

export interface ComplianceFramework {
  id: number;
  name: string;
  version?: string;
  description?: string;
  is_builtin: boolean;
  is_active: boolean;
  created_at: number;
}

export interface ComplianceControl {
  id: number;
  framework_id: number;
  framework_name: string;
  control_id: string;
  title: string;
  description?: string;
  category?: string;
  sort_order: number;
}

export interface FindingComplianceMapping {
  id: number;
  finding_id: number;
  control_id: number;
  control: ComplianceControl;
  notes?: string;
}

export interface FrameworkInput {
  name: string;
  version?: string;
  description?: string;
}

export interface ControlInput {
  framework_id: number;
  control_id: string;
  title: string;
  description?: string;
  category?: string;
}

export interface MappingInput {
  finding_id: number;
  control_id: number;
  notes?: string;
}

export async function listFrameworks(): Promise<ComplianceFramework[]> {
  return invoke<ComplianceFramework[]>("list_frameworks");
}

export async function listControls(
  frameworkId: number,
): Promise<ComplianceControl[]> {
  return invoke<ComplianceControl[]>("list_controls", {
    framework_id: frameworkId,
  });
}

export async function createFramework(input: FrameworkInput): Promise<number> {
  return invoke<number>("create_framework", { input });
}

export async function createControl(input: ControlInput): Promise<number> {
  return invoke<number>("create_control", { input });
}

export async function mapFindingToControl(
  input: MappingInput,
): Promise<number> {
  return invoke<number>("map_finding_to_control", { input });
}

export async function getFindingMappings(
  findingId: number,
): Promise<FindingComplianceMapping[]> {
  return invoke<FindingComplianceMapping[]>("get_finding_mappings", {
    finding_id: findingId,
  });
}

export async function getEngagementComplianceCoverage(
  engagementId: number,
): Promise<Record<string, unknown>[]> {
  return invoke<Record<string, unknown>[]>(
    "get_engagement_compliance_coverage",
    { engagement_id: engagementId },
  );
}

export async function removeComplianceMapping(
  mappingId: number,
): Promise<void> {
  return invoke<void>("remove_compliance_mapping", { mapping_id: mappingId });
}
