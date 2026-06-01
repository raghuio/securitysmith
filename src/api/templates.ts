import { invoke } from "@tauri-apps/api/core";

export type TemplateCategory =
  | "finding"
  | "requirements"
  | "checklist"
  | "email"
  | "status_report"
  | "engagement_status";

export interface TemplateSummary {
  id: number;
  name: string;
  category: string;
  subcategory: string;
  tags: string[];
  is_builtin: boolean;
  created_at: number;
  updated_at: number;
}

export interface Template extends TemplateSummary {
  content: string;
  is_active: boolean;
}

export interface TemplateInput {
  name: string;
  category: TemplateCategory;
  subcategory: string;
  content: string;
  tags?: string[];
}

export async function listTemplates(
  category?: string,
  subcategory?: string,
  search?: string,
): Promise<TemplateSummary[]> {
  return invoke<TemplateSummary[]>("list_templates", {
    category: category || null,
    subcategory: subcategory || null,
    search: search || null,
  });
}

export async function getTemplate(id: number): Promise<Template> {
  return invoke<Template>("get_template", { id });
}

export async function createTemplate(input: TemplateInput): Promise<number> {
  return invoke<number>("create_template", { input });
}

export async function updateTemplate(
  id: number,
  updates: Partial<Pick<TemplateInput, "name" | "content" | "tags">>,
): Promise<void> {
  return invoke<void>("update_template", {
    id,
    name: updates.name || null,
    content: updates.content || null,
    tags: updates.tags || null,
  });
}

export async function duplicateTemplate(id: number): Promise<number> {
  return invoke<number>("duplicate_template", { id });
}

export async function deleteTemplate(id: number): Promise<void> {
  return invoke<void>("delete_template", { id });
}

export async function saveFindingAsTemplate(
  findingId: number,
  name: string,
  tags?: string[],
): Promise<number> {
  return invoke<number>("save_finding_as_template", {
    findingId,
    name,
    tags: tags || null,
  });
}
