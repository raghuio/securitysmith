import { useState, useEffect } from "react";
import {
  Button,
  Drawer,
  Stack,
  Textarea,
  TextInput,
  Text,
  Alert,
  Select,
  TagsInput,
  NumberInput,
  Group,
  ActionIcon,
  Paper,
} from "@mantine/core";
import { IconTrash, IconPlus } from "@tabler/icons-react";
import { AttachmentUploader } from "./AttachmentUploader";
import { AttachmentGallery } from "./AttachmentGallery";
import { ComplianceView } from "./ComplianceView";
import { createFinding, updateFinding } from "../api/findings";
import { listTemplates, getTemplate } from "../api/templates";
import type { TemplateSummary } from "../api/templates";
import type {
  Finding,
  FindingInput,
  Severity,
  AffectedEndpoint,
  Evidence,
  ImpactItem,
  RemediationItem,
  Reference,
} from "../api/findings";

interface Props {
  opened: boolean;
  finding: Finding | null;
  engagementId: number;
  onClose: () => void;
  onSaved: () => void;
}

const SEVERITY_OPTIONS: { value: Severity; label: string }[] = [
  { value: "critical", label: "Critical" },
  { value: "high", label: "High" },
  { value: "medium", label: "Medium" },
  { value: "low", label: "Low" },
  { value: "informational", label: "Informational" },
];

export function FindingForm({
  opened,
  finding,
  engagementId,
  onClose,
  onSaved,
}: Props) {
  const isEdit = !!finding;
  const [title, setTitle] = useState("");
  const [severity, setSeverity] = useState<Severity>("high");
  const [overview, setOverview] = useState("");
  const [summary, setSummary] = useState("");
  const [stepsToReproduce, setStepsToReproduce] = useState("");
  const [endpoints, setEndpoints] = useState<AffectedEndpoint[]>([
    { method: "GET", path: "", description: "" },
  ]);
  const [evidence, setEvidence] = useState<Evidence[]>([
    { title: "", request: "", response: "" },
  ]);
  const [impactItems, setImpactItems] = useState<ImpactItem[]>([
    { title: "", explanation: "" },
  ]);
  const [remediationItems, setRemediationItems] = useState<RemediationItem[]>([
    { action: "", fix: "", code_snippet: undefined },
  ]);
  const [references, setReferences] = useState<Reference[]>([]);
  const [cvssScore, setCvssScore] = useState<number | "">("");
  const [owaspCategory, setOwaspCategory] = useState("");
  const [cweId, setCweId] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [notes, setNotes] = useState("");
  const [templates, setTemplates] = useState<TemplateSummary[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (finding) {
      setTitle(finding.title);
      setSeverity(finding.severity);
      setOverview(finding.overview);
      setSummary(finding.summary);
      setStepsToReproduce(finding.steps_to_reproduce);
      setEndpoints(
        finding.affected_endpoints.length > 0
          ? finding.affected_endpoints
          : [{ method: "GET", path: "", description: "" }],
      );
      setEvidence(
        finding.evidence.length > 0
          ? finding.evidence
          : [{ title: "", request: "", response: "" }],
      );
      setImpactItems(
        finding.impact_items.length > 0
          ? finding.impact_items
          : [{ title: "", explanation: "" }],
      );
      setRemediationItems(
        finding.remediation_items.length > 0
          ? finding.remediation_items
          : [{ action: "", fix: "", code_snippet: undefined }],
      );
      setReferences(finding.references);
      setCvssScore(finding.cvss_score ?? "");
      setOwaspCategory(finding.owasp_category ?? "");
      setCweId(finding.cwe_id ?? "");
      setTags(finding.tags);
      setNotes(finding.notes ?? "");
    } else {
      setTitle("");
      setSeverity("high");
      setOverview("");
      setSummary("");
      setStepsToReproduce("");
      setEndpoints([{ method: "GET", path: "", description: "" }]);
      setEvidence([{ title: "", request: "", response: "" }]);
      setImpactItems([{ title: "", explanation: "" }]);
      setRemediationItems([{ action: "", fix: "", code_snippet: undefined }]);
      setReferences([]);
      setCvssScore("");
      setOwaspCategory("");
      setCweId("");
      setTags([]);
      setNotes("");
    }
    setError(null);
  }, [finding]);

  useEffect(() => {
    if (opened && !finding) {
      listTemplates("finding", undefined, undefined)
        .then(setTemplates)
        .catch(console.error);
    }
  }, [opened, finding]);

  const applyTemplate = async (templateId: string) => {
    if (!templateId) return;
    try {
      const t = await getTemplate(Number(templateId));
      const data = JSON.parse(t.content) as Record<string, unknown>;
      if (data.title) setTitle(String(data.title));
      if (data.severity) setSeverity(String(data.severity) as Severity);
      if (data.overview) setOverview(String(data.overview));
      if (data.summary) setSummary(String(data.summary));
      if (data.steps_to_reproduce)
        setStepsToReproduce(String(data.steps_to_reproduce));
      if (Array.isArray(data.affected_endpoints))
        setEndpoints(data.affected_endpoints as AffectedEndpoint[]);
      if (Array.isArray(data.evidence))
        setEvidence(data.evidence as Evidence[]);
      if (Array.isArray(data.impact_items))
        setImpactItems(data.impact_items as ImpactItem[]);
      if (Array.isArray(data.remediation_items))
        setRemediationItems(data.remediation_items as RemediationItem[]);
      if (Array.isArray(data.references))
        setReferences(data.references as Reference[]);
      if (data.cvss_score) setCvssScore(Number(data.cvss_score));
      if (data.owasp_category) setOwaspCategory(String(data.owasp_category));
      if (data.cwe_id) setCweId(String(data.cwe_id));
      if (Array.isArray(data.tags)) setTags(data.tags as string[]);
      if (data.notes) setNotes(String(data.notes));
    } catch (e) {
      console.error("Failed to apply template:", e);
    }
  };

  const validate = (): boolean => {
    if (!title.trim()) {
      setError("Title is required.");
      return false;
    }
    if (!overview.trim()) {
      setError("Overview is required.");
      return false;
    }
    if (!summary.trim()) {
      setError("Summary is required.");
      return false;
    }
    if (!stepsToReproduce.trim()) {
      setError("Steps to reproduce are required.");
      return false;
    }
    const validEndpoints = endpoints.filter(
      (e) => e.method.trim() && e.path.trim(),
    );
    if (validEndpoints.length === 0) {
      setError("At least one affected endpoint is required.");
      return false;
    }
    const validEvidence = evidence.filter(
      (e) => e.title.trim() && e.request.trim(),
    );
    if (validEvidence.length === 0) {
      setError("At least one evidence entry is required.");
      return false;
    }
    const validImpact = impactItems.filter((i) => i.title.trim());
    if (validImpact.length === 0) {
      setError("At least one impact item is required.");
      return false;
    }
    const validRemediation = remediationItems.filter((r) => r.action.trim());
    if (validRemediation.length === 0) {
      setError("At least one remediation item is required.");
      return false;
    }
    return true;
  };

  const handleSubmit = async () => {
    setError(null);
    if (!validate()) return;

    setLoading(true);
    try {
      const input: FindingInput = {
        engagement_id: engagementId,
        title: title.trim(),
        severity,
        overview: overview.trim(),
        summary: summary.trim(),
        affected_endpoints: endpoints.filter(
          (e) => e.method.trim() && e.path.trim(),
        ),
        evidence: evidence.filter((e) => e.title.trim() && e.request.trim()),
        impact_items: impactItems.filter((i) => i.title.trim()),
        remediation_items: remediationItems.filter((r) => r.action.trim()),
        steps_to_reproduce: stepsToReproduce.trim(),
        cvss_score: typeof cvssScore === "number" ? cvssScore : undefined,
        owasp_category: owaspCategory.trim() || undefined,
        cwe_id: cweId.trim() || undefined,
        references: references.length > 0 ? references : undefined,
        tags: tags.length > 0 ? tags : undefined,
        notes: notes.trim() || undefined,
      };

      if (isEdit && finding) {
        await updateFinding(finding.id, input);
      } else {
        await createFinding(input);
      }
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const renderArraySection = <T,>(
    label: string,
    items: T[],
    setItems: (items: T[]) => void,
    renderItem: (item: T, index: number) => React.ReactNode,
    addItem: () => T,
  ) => (
    <Stack gap="xs">
      <Text fw={600} size="sm">
        {label}
      </Text>
      {items.map((item, idx) => (
        <Paper key={idx} withBorder p="sm" radius="sm">
          <Group justify="space-between">
            <div style={{ flex: 1 }}>{renderItem(item, idx)}</div>
            <ActionIcon
              variant="light"
              color="red"
              size="sm"
              onClick={() => setItems(items.filter((_, i) => i !== idx))}
            >
              <IconTrash size={14} />
            </ActionIcon>
          </Group>
        </Paper>
      ))}
      <Button
        leftSection={<IconPlus size={14} />}
        variant="light"
        size="xs"
        onClick={() => setItems([...items, addItem()])}
      >
        Add
      </Button>
    </Stack>
  );

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={isEdit ? "Edit Finding" : "New Finding"}
      position="right"
      size="xl"
    >
      <Stack gap="md">
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}

        {!isEdit && templates.length > 0 && (
          <Select
            label="Apply Template (optional)"
            placeholder="Choose a finding template..."
            value={selectedTemplateId}
            onChange={(val) => {
              setSelectedTemplateId(val || "");
              if (val) applyTemplate(val);
            }}
            data={templates.map((t) => ({
              value: String(t.id),
              label: t.name,
            }))}
            clearable
          />
        )}

        <TextInput
          label="Title"
          placeholder="SQL Injection in login form"
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
          required
          autoFocus={!isEdit}
        />

        <Select
          label="Severity"
          data={SEVERITY_OPTIONS}
          value={severity}
          onChange={(val) => setSeverity((val as Severity) ?? "high")}
          required
        />

        <Group grow>
          <TextInput
            label="OWASP Category"
            placeholder="A03:2021"
            value={owaspCategory}
            onChange={(e) => setOwaspCategory(e.currentTarget.value)}
          />
          <TextInput
            label="CWE ID"
            placeholder="CWE-89"
            value={cweId}
            onChange={(e) => setCweId(e.currentTarget.value)}
          />
          <NumberInput
            label="CVSS Score"
            placeholder="7.5"
            min={0}
            max={10}
            decimalScale={1}
            value={cvssScore === "" ? undefined : cvssScore}
            onChange={(val: string | number) =>
              setCvssScore(val === "" ? "" : Number(val))
            }
          />
        </Group>

        <Textarea
          label="Overview"
          placeholder="One-line summary..."
          value={overview}
          onChange={(e) => setOverview(e.currentTarget.value)}
          minRows={2}
          maxRows={4}
          required
        />

        <Textarea
          label="Summary"
          placeholder="Detailed technical explanation..."
          value={summary}
          onChange={(e) => setSummary(e.currentTarget.value)}
          minRows={4}
          maxRows={12}
          required
        />

        {renderArraySection(
          "Affected Endpoints",
          endpoints,
          setEndpoints,
          (ep, idx) => (
            <Stack gap="xs">
              <Group grow>
                <TextInput
                  placeholder="GET"
                  value={ep.method}
                  onChange={(e) => {
                    const copy = [...endpoints];
                    copy[idx].method = e.currentTarget.value;
                    setEndpoints(copy);
                  }}
                />
                <TextInput
                  placeholder="/api/login"
                  value={ep.path}
                  onChange={(e) => {
                    const copy = [...endpoints];
                    copy[idx].path = e.currentTarget.value;
                    setEndpoints(copy);
                  }}
                />
              </Group>
              <TextInput
                placeholder="Description"
                value={ep.description}
                onChange={(e) => {
                  const copy = [...endpoints];
                  copy[idx].description = e.currentTarget.value;
                  setEndpoints(copy);
                }}
              />
            </Stack>
          ),
          () => ({ method: "GET", path: "", description: "" }),
        )}

        {renderArraySection(
          "Evidence",
          evidence,
          setEvidence,
          (ev, idx) => (
            <Stack gap="xs">
              <TextInput
                placeholder="Evidence title"
                value={ev.title}
                onChange={(e) => {
                  const copy = [...evidence];
                  copy[idx].title = e.currentTarget.value;
                  setEvidence(copy);
                }}
              />
              <Textarea
                placeholder="Request"
                value={ev.request}
                onChange={(e) => {
                  const copy = [...evidence];
                  copy[idx].request = e.currentTarget.value;
                  setEvidence(copy);
                }}
                minRows={3}
              />
              <Textarea
                placeholder="Response"
                value={ev.response}
                onChange={(e) => {
                  const copy = [...evidence];
                  copy[idx].response = e.currentTarget.value;
                  setEvidence(copy);
                }}
                minRows={3}
              />
            </Stack>
          ),
          () => ({ title: "", request: "", response: "" }),
        )}

        {renderArraySection(
          "Impact",
          impactItems,
          setImpactItems,
          (item, idx) => (
            <Stack gap="xs">
              <TextInput
                placeholder="Impact title"
                value={item.title}
                onChange={(e) => {
                  const copy = [...impactItems];
                  copy[idx].title = e.currentTarget.value;
                  setImpactItems(copy);
                }}
              />
              <Textarea
                placeholder="Explanation"
                value={item.explanation}
                onChange={(e) => {
                  const copy = [...impactItems];
                  copy[idx].explanation = e.currentTarget.value;
                  setImpactItems(copy);
                }}
                minRows={2}
              />
            </Stack>
          ),
          () => ({ title: "", explanation: "" }),
        )}

        {renderArraySection(
          "Remediation",
          remediationItems,
          setRemediationItems,
          (item, idx) => (
            <Stack gap="xs">
              <TextInput
                placeholder="Action"
                value={item.action}
                onChange={(e) => {
                  const copy = [...remediationItems];
                  copy[idx].action = e.currentTarget.value;
                  setRemediationItems(copy);
                }}
              />
              <Textarea
                placeholder="Fix description"
                value={item.fix}
                onChange={(e) => {
                  const copy = [...remediationItems];
                  copy[idx].fix = e.currentTarget.value;
                  setRemediationItems(copy);
                }}
                minRows={2}
              />
              <Textarea
                placeholder="Code snippet (optional)"
                value={item.code_snippet ?? ""}
                onChange={(e) => {
                  const copy = [...remediationItems];
                  copy[idx].code_snippet = e.currentTarget.value || undefined;
                  setRemediationItems(copy);
                }}
                minRows={2}
              />
            </Stack>
          ),
          () => ({ action: "", fix: "", code_snippet: undefined }),
        )}

        <Textarea
          label="Steps to Reproduce"
          placeholder="How to trigger the issue..."
          value={stepsToReproduce}
          onChange={(e) => setStepsToReproduce(e.currentTarget.value)}
          minRows={4}
          maxRows={12}
          required
        />

        {renderArraySection(
          "References",
          references,
          setReferences,
          (ref, idx) => (
            <Group grow>
              <TextInput
                placeholder="Title"
                value={ref.title}
                onChange={(e) => {
                  const copy = [...references];
                  copy[idx].title = e.currentTarget.value;
                  setReferences(copy);
                }}
              />
              <TextInput
                placeholder="https://..."
                value={ref.url}
                onChange={(e) => {
                  const copy = [...references];
                  copy[idx].url = e.currentTarget.value;
                  setReferences(copy);
                }}
              />
            </Group>
          ),
          () => ({ title: "", url: "" }),
        )}

        <TagsInput
          label="Tags"
          placeholder="Add a tag and press Enter"
          value={tags}
          onChange={setTags}
          splitChars={[","]}
        />

        <Textarea
          label="Notes"
          placeholder="Any additional notes..."
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          minRows={2}
          maxRows={6}
        />

        {isEdit && finding && (
          <>
            <Stack gap="sm" mt="md">
              <Text fw={600}>Attachments</Text>
              <AttachmentUploader
                entityType="finding"
                entityId={finding.id}
                onUploaded={() => {}}
              />
              <AttachmentGallery entityType="finding" entityId={finding.id} />
            </Stack>
            <ComplianceView findingId={finding.id} />
          </>
        )}

        <Stack gap="xs" mt="md">
          <Button onClick={handleSubmit} loading={loading}>
            {isEdit ? "Save Changes" : "Create Finding"}
          </Button>
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
        </Stack>
      </Stack>
    </Drawer>
  );
}
