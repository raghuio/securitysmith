import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Group,
  Progress,
  Select,
  Stack,
  Text,
  Title,
  Badge,
  Checkbox,
} from "@mantine/core";
import {
  listChecklists,
  assignChecklistToEngagement,
  getEngagementChecklist,
  updateEngagementChecklistItem,
  getChecklistCoverage,
  type Checklist,
  type EngagementChecklistItem,
} from "../api";

export function ChecklistView({ engagementId }: { engagementId: number }) {
  const [checklists, setChecklists] = useState<Checklist[]>([]);
  const [selectedChecklistId, setSelectedChecklistId] = useState<number | null>(
    null,
  );
  const [items, setItems] = useState<EngagementChecklistItem[]>([]);
  const [coverage, setCoverage] = useState<[number, number, number] | null>(
    null,
  );

  const load = async () => {
    try {
      const data = await listChecklists();
      setChecklists(data);
      if (engagementId) {
        const assigned = await getEngagementChecklist(engagementId);
        setItems(assigned);
        const cov = await getChecklistCoverage(engagementId);
        setCoverage(cov);
      }
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [engagementId]);

  const handleAssign = async () => {
    if (!selectedChecklistId || !engagementId) return;
    try {
      await assignChecklistToEngagement(engagementId, selectedChecklistId);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleUpdateStatus = async (id: number, status: string) => {
    try {
      await updateEngagementChecklistItem(id, status);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const grouped = items.reduce<Record<string, EngagementChecklistItem[]>>(
    (acc, item) => {
      const cat = item.checklist_item.category;
      if (!acc[cat]) acc[cat] = [];
      acc[cat].push(item);
      return acc;
    },
    {},
  );

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={4}>Methodology Checklist</Title>
        {coverage && (
          <Badge size="lg">
            {coverage[1]}/{coverage[2]} ({coverage[0].toFixed(0)}%)
          </Badge>
        )}
      </Group>

      {items.length === 0 && (
        <Group gap="sm">
          <Select
            placeholder="Select checklist"
            data={checklists.map((c) => ({
              value: String(c.id),
              label: c.name,
            }))}
            value={selectedChecklistId ? String(selectedChecklistId) : null}
            onChange={(v) => setSelectedChecklistId(v ? Number(v) : null)}
            w={240}
          />
          <Button onClick={handleAssign}>Assign</Button>
        </Group>
      )}

      <Stack gap="md">
        {Object.entries(grouped).map(([category, catItems]) => {
          const tested = catItems.filter((i) =>
            ["tested", "finding_created", "not_applicable"].includes(i.status),
          ).length;
          return (
            <Card key={category} withBorder padding="sm" radius="md">
              <Group justify="space-between" mb="xs">
                <Text fw={600}>{category}</Text>
                <Text size="sm" c="dimmed">
                  {tested}/{catItems.length}
                </Text>
              </Group>
              <Progress
                value={(tested / catItems.length) * 100}
                size="sm"
                mb="sm"
              />
              <Stack gap="xs">
                {catItems.map((item) => (
                  <Group key={item.id} justify="space-between">
                    <Group gap="sm">
                      <Checkbox
                        checked={[
                          "tested",
                          "finding_created",
                          "not_applicable",
                        ].includes(item.status)}
                        onChange={(e) =>
                          handleUpdateStatus(
                            item.id,
                            e.currentTarget.checked ? "tested" : "not_started",
                          )
                        }
                      />
                      <Stack gap={0}>
                        <Text size="sm">
                          {item.checklist_item.test_id
                            ? `${item.checklist_item.test_id} · `
                            : ""}
                          {item.checklist_item.name}
                        </Text>
                        {item.notes && (
                          <Text size="xs" c="dimmed">
                            {item.notes}
                          </Text>
                        )}
                      </Stack>
                    </Group>
                    <Badge
                      size="sm"
                      variant="light"
                      color={
                        item.status === "tested" ||
                        item.status === "finding_created"
                          ? "green"
                          : item.status === "in_progress"
                            ? "yellow"
                            : item.status === "not_applicable"
                              ? "gray"
                              : "red"
                      }
                    >
                      {item.status.replace("_", " ")}
                    </Badge>
                  </Group>
                ))}
              </Stack>
            </Card>
          );
        })}
      </Stack>
    </Stack>
  );
}

export function ChecklistEditor() {
  const [checklists, setChecklists] = useState<Checklist[]>([]);

  useEffect(() => {
    listChecklists().then(setChecklists).catch(console.error);
  }, []);

  return (
    <Stack gap="md">
      <Title order={3}>Checklists</Title>
      {checklists.map((c) => (
        <Card key={c.id} withBorder padding="sm" radius="md">
          <Group justify="space-between">
            <Text fw={600}>{c.name}</Text>
            {c.is_builtin && <Badge size="sm">Built-in</Badge>}
          </Group>
          {c.description && (
            <Text size="sm" c="dimmed">
              {c.description}
            </Text>
          )}
        </Card>
      ))}
    </Stack>
  );
}
