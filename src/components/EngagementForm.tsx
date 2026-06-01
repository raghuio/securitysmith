import { useState, useEffect, useMemo } from "react";
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
  Group,
  Autocomplete,
  Checkbox,
  NumberInput,
} from "@mantine/core";
import { createEngagement, updateEngagement } from "../api/engagements";
import type {
  Engagement,
  EngagementInput,
  EngagementStatus,
} from "../api/engagements";
import { CredentialList } from "./CredentialList";
import { ScopeEditor } from "./ScopeEditor";
import { TimeTracker } from "./TimeTracker";
import { ChecklistView } from "./ChecklistView";
import { RetestView } from "./RetestView";
import type { Client } from "../api/clients";

interface Props {
  opened: boolean;
  engagement: Engagement | null;
  clients: Client[];
  preselectedClientId?: number | null;
  onClose: () => void;
  onSaved: () => void;
}

const STATUS_OPTIONS: { value: EngagementStatus; label: string }[] = [
  { value: "planned", label: "Planned" },
  { value: "scheduled", label: "Scheduled" },
  { value: "active", label: "Active" },
  { value: "paused", label: "Paused" },
  { value: "completed", label: "Completed" },
];

const TARGET_AREA_SUGGESTIONS = [
  "Web",
  "API",
  "LLM",
  "Network",
  "Mobile",
  "Cloud",
  "Desktop",
  "Thick Client",
  "Wireless",
  "Source Code",
  "Configuration",
  "General",
  "Retest",
  "Other",
];

const ASSESSMENT_KIND_SUGGESTIONS = [
  "Pentest",
  "VA",
  "VAPT",
  "Security Review",
  "Architecture Review",
  "Threat Model",
  "Retest",
  "Advisory",
  "Other",
];

const ACCESS_MODEL_SUGGESTIONS = [
  "Authenticated",
  "Unauthenticated",
  "Mixed",
  "Not Applicable",
  "Other",
];

export function EngagementForm({
  opened,
  engagement,
  clients,
  preselectedClientId,
  onClose,
  onSaved,
}: Props) {
  const isEdit = !!engagement;

  const [clientId, setClientId] = useState<number | "">("");
  const [name, setName] = useState("");
  const [targetArea, setTargetArea] = useState("");
  const [assessmentKind, setAssessmentKind] = useState("");
  const [accessModel, setAccessModel] = useState("");
  const [engagementType, setEngagementType] = useState("");
  const [status, setStatus] = useState<EngagementStatus>("planned");
  const [startDate, setStartDate] = useState<Date | null>(null);
  const [endDate, setEndDate] = useState<Date | null>(null);
  const [scopeSummary, setScopeSummary] = useState("");
  const [objectives, setObjectives] = useState<string[]>([]);
  const [notes, setNotes] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [paymentRequired, setPaymentRequired] = useState(false);
  const [budgetedHours, setBudgetedHours] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clientSelectData = useMemo(
    () => clients.map((c) => ({ value: String(c.id), label: c.name })),
    [clients],
  );

  useEffect(() => {
    if (opened) {
      setError(null);
    }
  }, [opened]);

  useEffect(() => {
    if (engagement) {
      setClientId(engagement.client_id);
      setName(engagement.name);
      setTargetArea(engagement.target_area);
      setAssessmentKind(engagement.assessment_kind);
      setAccessModel(engagement.access_model);
      setEngagementType(engagement.engagement_type);
      setStatus(engagement.status);
      setStartDate(
        engagement.start_date
          ? new Date(engagement.start_date + "T00:00:00")
          : null,
      );
      setEndDate(
        engagement.end_date
          ? new Date(engagement.end_date + "T00:00:00")
          : null,
      );
      setScopeSummary(engagement.scope_summary ?? "");
      setObjectives(engagement.objectives);
      setNotes(engagement.notes ?? "");
      setTags(engagement.tags);
      setPaymentRequired(engagement.payment_required);
      setBudgetedHours(
        engagement.budgeted_hours !== null
          ? String(engagement.budgeted_hours)
          : "",
      );
    } else {
      setClientId(preselectedClientId ?? "");
      setName("");
      setTargetArea("");
      setAssessmentKind("");
      setAccessModel("");
      setEngagementType("");
      setStatus("planned");
      setStartDate(null);
      setEndDate(null);
      setScopeSummary("");
      setObjectives([]);
      setNotes("");
      setTags([]);
      setPaymentRequired(false);
      setBudgetedHours("");
    }
  }, [engagement, preselectedClientId]);

  useEffect(() => {
    if (!isEdit && targetArea && assessmentKind && accessModel && name) {
      const composed = `${targetArea} ${assessmentKind}`;
      setEngagementType(composed);
    }
  }, [targetArea, assessmentKind, accessModel, name, isEdit]);

  const validate = (): boolean => {
    if (!clientId) {
      setError("Client is required.");
      return false;
    }
    if (!name.trim()) {
      setError("Engagement name is required.");
      return false;
    }
    if (name.trim().length > 255) {
      setError("Engagement name must be 255 characters or fewer.");
      return false;
    }
    if (!targetArea.trim()) {
      setError("Target area is required.");
      return false;
    }
    if (!assessmentKind.trim()) {
      setError("Assessment kind is required.");
      return false;
    }
    if (!accessModel.trim()) {
      setError("Access model is required.");
      return false;
    }
    if (!engagementType.trim()) {
      setError("Engagement type is required.");
      return false;
    }
    if (scopeSummary.length > 5000) {
      setError("Scope summary must be 5,000 characters or fewer.");
      return false;
    }
    if (notes.length > 20000) {
      setError("Notes must be 20,000 characters or fewer.");
      return false;
    }
    if (startDate && endDate && endDate < startDate) {
      setError("End date cannot be before start date.");
      return false;
    }
    return true;
  };

  const toIsoDate = (d: Date | null): string | undefined => {
    if (!d) return undefined;
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  };

  const handleSubmit = async () => {
    setError(null);
    if (!validate()) return;

    setLoading(true);
    try {
      const input: EngagementInput = {
        client_id: Number(clientId),
        name: name.trim(),
        target_area: targetArea.trim(),
        assessment_kind: assessmentKind.trim(),
        access_model: accessModel.trim(),
        engagement_type: engagementType.trim(),
        status,
        start_date: toIsoDate(startDate),
        end_date: toIsoDate(endDate),
        scope_summary: scopeSummary || undefined,
        objectives: objectives.length > 0 ? objectives : undefined,
        notes: notes || undefined,
        tags: tags.length > 0 ? tags : undefined,
        payment_required: paymentRequired,
        budgeted_hours:
          budgetedHours.trim() === "" ? undefined : Number(budgetedHours),
      };

      if (isEdit && engagement) {
        await updateEngagement(engagement.id, input);
      } else {
        await createEngagement(input);
      }
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={isEdit ? "Edit Engagement" : "New Engagement"}
      position="right"
      size="md"
    >
      <Stack>
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}

        <Select
          label="Client"
          placeholder="Select a client"
          data={clientSelectData}
          value={clientId ? String(clientId) : null}
          onChange={(val) => setClientId(val ? Number(val) : "")}
          disabled={isEdit && !!preselectedClientId}
          required
          searchable
        />

        <TextInput
          label="Name"
          placeholder="Q3 Web App Pentest"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          required
          autoFocus={!isEdit}
        />

        <Autocomplete
          label="Target Area"
          placeholder="Web"
          data={TARGET_AREA_SUGGESTIONS}
          value={targetArea}
          onChange={(val) => setTargetArea(val ?? "")}
          required
        />

        <Autocomplete
          label="Assessment Kind"
          placeholder="Pentest"
          data={ASSESSMENT_KIND_SUGGESTIONS}
          value={assessmentKind}
          onChange={(val) => setAssessmentKind(val ?? "")}
          required
        />

        <Autocomplete
          label="Access Model"
          placeholder="Authenticated"
          data={ACCESS_MODEL_SUGGESTIONS}
          value={accessModel}
          onChange={(val) => setAccessModel(val ?? "")}
          required
        />

        <TextInput
          label="Engagement Type (display label)"
          placeholder="Web Pentest"
          value={engagementType}
          onChange={(e) => setEngagementType(e.currentTarget.value)}
          required
        />

        <Select
          label="Status"
          data={STATUS_OPTIONS}
          value={status}
          onChange={(val) => setStatus((val as EngagementStatus) ?? "planned")}
          required
        />

        {isEdit && engagement && (
          <Stack gap="xs">
            <Text size="sm" fw={600}>
              Scheduling Gates
            </Text>
            <Text
              size="sm"
              c={engagement.credentials_ready ? "green" : "dimmed"}
            >
              {engagement.credentials_ready
                ? "✓ Credentials gate passed"
                : "✗ Credentials not ready — mark all credentials as Working"}
            </Text>
            {engagement.payment_required && (
              <Text
                size="sm"
                c={engagement.payment_cleared ? "green" : "dimmed"}
              >
                {engagement.payment_cleared
                  ? "✓ Payment gate passed"
                  : "✗ Payment not cleared — advance must be received"}
              </Text>
            )}
          </Stack>
        )}

        <Checkbox
          label="Payment required for this engagement"
          checked={paymentRequired}
          onChange={(e) => setPaymentRequired(e.currentTarget.checked)}
        />

        <NumberInput
          label="Budgeted hours"
          description="Optional. Used by Time Tracking to compute utilization."
          placeholder="e.g. 40"
          min={0}
          max={10000}
          step={1}
          decimalScale={2}
          value={budgetedHours === "" ? "" : Number(budgetedHours)}
          onChange={(v) =>
            setBudgetedHours(typeof v === "number" ? String(v) : "")
          }
        />

        <Group grow>
          <TextInput
            label="Start Date"
            placeholder="YYYY-MM-DD"
            type="date"
            value={startDate ? (toIsoDate(startDate) ?? "") : ""}
            onChange={(e) => {
              const v = e.currentTarget.value;
              setStartDate(v ? new Date(v + "T00:00:00") : null);
            }}
          />
          <TextInput
            label="End Date"
            placeholder="YYYY-MM-DD"
            type="date"
            value={endDate ? (toIsoDate(endDate) ?? "") : ""}
            onChange={(e) => {
              const v = e.currentTarget.value;
              setEndDate(v ? new Date(v + "T00:00:00") : null);
            }}
          />
        </Group>

        <Textarea
          label="Scope Summary"
          placeholder="Brief scope description..."
          value={scopeSummary}
          onChange={(e) => setScopeSummary(e.currentTarget.value)}
          minRows={3}
          maxRows={8}
        />
        <Text size="xs" c="dimmed">
          {scopeSummary.length} / 5,000
        </Text>

        <TagsInput
          label="Objectives"
          placeholder="Add an objective and press Enter"
          value={objectives}
          onChange={setObjectives}
          splitChars={[","]}
        />

        <Textarea
          label="Notes"
          placeholder="Any notes about this engagement..."
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          minRows={3}
          maxRows={8}
        />
        <Text size="xs" c="dimmed">
          {notes.length} / 20,000
        </Text>

        <TagsInput
          label="Tags"
          placeholder="Add a tag and press Enter"
          value={tags}
          onChange={setTags}
          splitChars={[","]}
        />

        {isEdit && engagement && (
          <>
            <CredentialList engagementId={engagement.id} refreshKey={0} />
            <ScopeEditor engagementId={engagement.id} />
            <TimeTracker engagementId={engagement.id} />
            <ChecklistView engagementId={engagement.id} />
            <RetestView engagementId={engagement.id} />
          </>
        )}

        <Stack gap="xs" mt="md">
          <Button onClick={handleSubmit} loading={loading}>
            {isEdit ? "Save Changes" : "Create Engagement"}
          </Button>
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
        </Stack>
      </Stack>
    </Drawer>
  );
}
