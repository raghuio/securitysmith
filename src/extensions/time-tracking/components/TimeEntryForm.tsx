import { useState } from "react";
import {
  Button,
  Card,
  Checkbox,
  Group,
  NumberInput,
  Select,
  Stack,
  TextInput,
} from "@mantine/core";

const ACTIVITIES = [
  { value: "testing", label: "Testing" },
  { value: "reporting", label: "Reporting" },
  { value: "scoping", label: "Scoping" },
  { value: "communication", label: "Communication" },
  { value: "remediation_support", label: "Remediation Support" },
  { value: "retest", label: "Retest" },
  { value: "admin", label: "Admin" },
  { value: "other", label: "Other" },
];

export function TimeEntryForm({
  prefillHours,
  onSave,
  onCancel,
}: {
  prefillHours?: number;
  onSave: (input: {
    entry_date: string;
    hours: number;
    description?: string;
    activity_type: string;
    is_billable?: boolean;
  }) => void;
  onCancel: () => void;
}) {
  const today = new Date().toISOString().split("T")[0];
  const [entryDate, setEntryDate] = useState(today);
  const [hours, setHours] = useState<number>(prefillHours || 1);
  const [description, setDescription] = useState("");
  const [activityType, setActivityType] = useState("testing");
  const [isBillable, setIsBillable] = useState(true);

  const handleSubmit = () => {
    onSave({
      entry_date: entryDate,
      hours: hours || 0,
      description: description || undefined,
      activity_type: activityType,
      is_billable: isBillable,
    });
  };

  return (
    <Card withBorder padding="sm" radius="md">
      <Stack gap="sm">
        <TextInput
          label="Date"
          type="date"
          value={entryDate}
          onChange={(e) => setEntryDate(e.currentTarget.value)}
        />
        <NumberInput
          label="Hours"
          value={hours}
          onChange={(v) => setHours(typeof v === "number" ? v : 0)}
          min={0.25}
          max={24}
          step={0.25}
          decimalScale={2}
        />
        <Select
          label="Activity"
          data={ACTIVITIES}
          value={activityType}
          onChange={(v) => v && setActivityType(v)}
        />
        <TextInput
          label="Description"
          value={description}
          onChange={(e) => setDescription(e.currentTarget.value)}
        />
        <Checkbox
          label="Billable"
          checked={isBillable}
          onChange={(e) => setIsBillable(e.currentTarget.checked)}
        />
        <Group justify="flex-end">
          <Button variant="default" size="xs" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="xs" onClick={handleSubmit}>
            Save
          </Button>
        </Group>
      </Stack>
    </Card>
  );
}
