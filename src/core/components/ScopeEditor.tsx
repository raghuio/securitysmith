import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Group,
  Select,
  Stack,
  Switch,
  Textarea,
  TextInput,
  Text,
  Title,
  Badge,
} from "@mantine/core";
import {
  listScopeItems,
  createScopeItem,
  updateScopeItem,
  deleteScopeItem,
  bulkImportScopeItems,
  exportScopeText,
  type ScopeItem,
  type ScopeItemInput,
} from "../api/scope";

const SCOPE_TYPES = [
  { value: "url", label: "URL" },
  { value: "ip", label: "IP" },
  { value: "ip_range", label: "IP Range" },
  { value: "cidr", label: "CIDR" },
  { value: "domain", label: "Domain" },
  { value: "subdomain", label: "Subdomain" },
  { value: "application", label: "Application" },
  { value: "api_endpoint", label: "API Endpoint" },
  { value: "host", label: "Host" },
  { value: "other", label: "Other" },
];

export function ScopeEditor({ engagementId }: { engagementId: number }) {
  const [items, setItems] = useState<ScopeItem[]>([]);
  const [bulkText, setBulkText] = useState("");
  const [showBulk, setShowBulk] = useState(false);
  const [editing, setEditing] = useState<ScopeItem | null>(null);

  const load = async () => {
    try {
      const data = await listScopeItems(engagementId);
      setItems(data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [engagementId]);

  const handleSave = async (input: ScopeItemInput) => {
    try {
      if (editing) {
        await updateScopeItem(editing.id, input);
      } else {
        await createScopeItem(input);
      }
      setEditing(null);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm("Delete this scope item?")) return;
    try {
      await deleteScopeItem(id);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleBulkImport = async () => {
    try {
      await bulkImportScopeItems(engagementId, bulkText);
      setBulkText("");
      setShowBulk(false);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleExport = async () => {
    try {
      const text = await exportScopeText(engagementId);
      const blob = new Blob([text], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `scope-${engagementId}.txt`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error(e);
    }
  };

  const inScope = items.filter((i) => i.is_in_scope);
  const outScope = items.filter((i) => !i.is_in_scope);

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={5}>Scope & Assets</Title>
        <Group gap="xs">
          <Button
            size="xs"
            variant="default"
            onClick={() => setShowBulk(!showBulk)}
          >
            Bulk Import
          </Button>
          <Button size="xs" variant="default" onClick={handleExport}>
            Export Text
          </Button>
        </Group>
      </Group>

      {showBulk && (
        <Card withBorder padding="sm" radius="md">
          <Textarea
            placeholder="Paste URLs/IPs (one per line)"
            minRows={4}
            value={bulkText}
            onChange={(e) => setBulkText(e.currentTarget.value)}
          />
          <Group justify="flex-end" mt="sm">
            <Button size="xs" onClick={handleBulkImport}>
              Import
            </Button>
          </Group>
        </Card>
      )}

      <ScopeItemForm
        engagementId={engagementId}
        item={editing}
        onSave={handleSave}
        onCancel={() => setEditing(null)}
      />

      <Stack gap="sm">
        <Text fw={600} size="sm">
          In Scope ({inScope.length})
        </Text>
        {inScope.map((item) => (
          <ScopeItemCard
            key={item.id}
            item={item}
            onEdit={() => setEditing(item)}
            onDelete={() => handleDelete(item.id)}
          />
        ))}
      </Stack>

      <Stack gap="sm">
        <Text fw={600} size="sm" c="red">
          Out of Scope ({outScope.length})
        </Text>
        {outScope.map((item) => (
          <ScopeItemCard
            key={item.id}
            item={item}
            onEdit={() => setEditing(item)}
            onDelete={() => handleDelete(item.id)}
          />
        ))}
      </Stack>
    </Stack>
  );
}

function ScopeItemCard({
  item,
  onEdit,
  onDelete,
}: {
  item: ScopeItem;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <Card withBorder padding="sm" radius="md">
      <Group justify="space-between">
        <Group gap="sm">
          <Badge size="sm" variant="light">
            {item.item_type}
          </Badge>
          <Text size="sm">{item.value}</Text>
          {item.environment && (
            <Badge size="sm" color="gray">
              {item.environment}
            </Badge>
          )}
        </Group>
        <Group gap="xs">
          <Button size="xs" variant="subtle" onClick={onEdit}>
            Edit
          </Button>
          <Button size="xs" variant="subtle" color="red" onClick={onDelete}>
            Delete
          </Button>
        </Group>
      </Group>
      {item.notes && (
        <Text size="xs" c="dimmed">
          {item.notes}
        </Text>
      )}
    </Card>
  );
}

function ScopeItemForm({
  engagementId,
  item,
  onSave,
  onCancel,
}: {
  engagementId: number;
  item?: ScopeItem | null;
  onSave: (input: ScopeItemInput) => void;
  onCancel: () => void;
}) {
  const [type, setType] = useState(item?.item_type || "url");
  const [value, setValue] = useState(item?.value || "");
  const [inScope, setInScope] = useState(item?.is_in_scope ?? true);
  const [environment, setEnvironment] = useState(item?.environment || "");
  const [notes, setNotes] = useState(item?.notes || "");

  const handleSubmit = () => {
    onSave({
      engagement_id: engagementId,
      item_type: type,
      value,
      is_in_scope: inScope,
      environment: environment || undefined,
      notes: notes || undefined,
    });
  };

  return (
    <Card withBorder padding="sm" radius="md">
      <Stack gap="sm">
        <Group grow>
          <Select
            label="Type"
            data={SCOPE_TYPES}
            value={type}
            onChange={(v) => v && setType(v)}
          />
          <TextInput
            label="Value"
            value={value}
            onChange={(e) => setValue(e.currentTarget.value)}
            required
          />
        </Group>
        <TextInput
          label="Environment"
          value={environment}
          onChange={(e) => setEnvironment(e.currentTarget.value)}
        />
        <TextInput
          label="Notes"
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
        />
        <Switch
          label="In Scope"
          checked={inScope}
          onChange={(e) => setInScope(e.currentTarget.checked)}
        />
        <Group justify="flex-end">
          <Button variant="default" size="xs" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="xs" onClick={handleSubmit}>
            {item ? "Update" : "Add"}
          </Button>
        </Group>
      </Stack>
    </Card>
  );
}
