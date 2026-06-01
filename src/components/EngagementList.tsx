import { useState, useEffect } from "react";
import {
  ActionIcon,
  Button,
  Group,
  Modal,
  Paper,
  Stack,
  Text,
  TextInput,
  Title,
  Select,
  Badge,
  Switch,
} from "@mantine/core";
import { TagGroup } from "./shared";
import {
  IconArrowLeft,
  IconEdit,
  IconSearch,
  IconTrash,
} from "@tabler/icons-react";
import {
  listEngagements,
  archiveEngagement,
  toggleEngagementGate,
  transitionEngagementStatus,
} from "../api/engagements";
import type { Engagement, EngagementStatus } from "../api/engagements";

interface Props {
  clientId?: number;
  onBack: () => void;
  onEdit: (engagement: Engagement) => void;
  onCreate: (clientId?: number) => void;
  onArchived: () => void;
  refreshKey: number;
}

const STATUS_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "All statuses" },
  { value: "planned", label: "Planned" },
  { value: "scheduled", label: "Scheduled" },
  { value: "active", label: "Active" },
  { value: "paused", label: "Paused" },
  { value: "completed", label: "Completed" },
];

const STATUS_COLORS: Record<EngagementStatus, string> = {
  planned: "gray",
  scheduled: "violet",
  active: "green",
  paused: "orange",
  completed: "blue",
};

export function EngagementList({
  clientId,
  onBack,
  onEdit,
  onCreate,
  onArchived,
  refreshKey,
}: Props) {
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<EngagementStatus | "">("");
  const [loading, setLoading] = useState(false);
  const [archiveTarget, setArchiveTarget] = useState<Engagement | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await listEngagements({
        clientId,
        search: search.trim() || undefined,
        status: (statusFilter || undefined) as EngagementStatus | undefined,
      });
      setEngagements(data);
    } catch (e) {
      console.error("Failed to load engagements:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, statusFilter, refreshKey]);

  const handleArchive = async () => {
    if (!archiveTarget) return;
    try {
      await archiveEngagement(archiveTarget.id);
      setArchiveTarget(null);
      onArchived();
      load();
    } catch (e) {
      console.error("Failed to archive engagement:", e);
    }
  };

  const handleToggleGate = async (
    id: number,
    gate: "credentials_ready" | "payment_cleared",
    value: boolean,
  ) => {
    try {
      await toggleEngagementGate(id, gate, value);
      load();
    } catch (e) {
      console.error(`Failed to toggle ${gate}:`, e);
    }
  };

  const handleTransition = async (id: number, newStatus: EngagementStatus) => {
    try {
      await transitionEngagementStatus(id, newStatus);
      load();
    } catch (e) {
      console.error("Failed to transition status:", e);
    }
  };

  return (
    <Stack gap="md" p="md">
      <Group justify="space-between" align="center">
        <Group gap="sm">
          <ActionIcon variant="light" onClick={onBack}>
            <IconArrowLeft size={18} />
          </ActionIcon>
          <Title order={4}>
            {clientId ? "Client Engagements" : "Engagements"}
          </Title>
        </Group>
        <Text size="sm" c="dimmed">
          {engagements.length}{" "}
          {engagements.length === 1 ? "engagement" : "engagements"}
        </Text>
      </Group>

      <Group grow align="flex-start">
        <TextInput
          placeholder="Search engagements..."
          leftSection={<IconSearch size={16} />}
          value={search}
          onChange={(e) => setSearch(e.currentTarget.value)}
          disabled={loading}
        />
        <Select
          placeholder="Filter by status"
          data={STATUS_OPTIONS}
          value={statusFilter}
          onChange={(val) =>
            setStatusFilter((val as EngagementStatus | "") ?? "")
          }
          disabled={loading}
          clearable
        />
      </Group>

      <Button
        leftSection={<IconEdit size={16} />}
        variant="light"
        onClick={() => onCreate(clientId)}
        disabled={!clientId}
      >
        Add engagement
      </Button>

      <Stack gap="xs">
        {engagements.length === 0 && !loading && (
          <Text c="dimmed" ta="center" py="xl">
            {search.trim() || statusFilter
              ? "No engagements match your filters."
              : "No engagements yet."}
          </Text>
        )}
        {engagements.map((engagement) => (
          <Paper key={engagement.id} withBorder shadow="xs" p="sm" radius="md">
            <Group justify="space-between" align="flex-start">
              <div style={{ flex: 1 }}>
                <Group gap="sm">
                  <Text fw={600}>{engagement.name}</Text>
                  <Badge color={STATUS_COLORS[engagement.status]} size="sm">
                    {engagement.status}
                  </Badge>
                </Group>
                <Text size="sm" c="dimmed">
                  {engagement.client_name} · {engagement.engagement_type}
                </Text>
                {(engagement.start_date || engagement.end_date) && (
                  <Text size="xs" c="dimmed">
                    {engagement.start_date ?? "Open start"} →{" "}
                    {engagement.end_date ?? "Open end"}
                  </Text>
                )}

                {/* Gate indicators */}
                <Group gap="xs" mt={4}>
                  <Badge
                    size="xs"
                    color={engagement.credentials_ready ? "green" : "red"}
                    variant="light"
                  >
                    Credentials{" "}
                    {engagement.credentials_ready ? "Ready" : "Not Ready"}
                  </Badge>
                  {engagement.payment_required && (
                    <Badge
                      size="xs"
                      color={engagement.payment_cleared ? "green" : "red"}
                      variant="light"
                    >
                      Payment{" "}
                      {engagement.payment_cleared ? "Cleared" : "Pending"}
                    </Badge>
                  )}
                </Group>

                {/* Gate toggles for planned engagements */}
                {engagement.status === "planned" && (
                  <Group gap="xs" mt={4}>
                    <Switch
                      size="xs"
                      label="Credentials ready"
                      checked={engagement.credentials_ready}
                      onChange={(e) =>
                        handleToggleGate(
                          engagement.id,
                          "credentials_ready",
                          e.currentTarget.checked,
                        )
                      }
                    />
                    {engagement.payment_required && (
                      <Switch
                        size="xs"
                        label="Payment cleared"
                        checked={engagement.payment_cleared}
                        onChange={(e) =>
                          handleToggleGate(
                            engagement.id,
                            "payment_cleared",
                            e.currentTarget.checked,
                          )
                        }
                      />
                    )}
                    <Button
                      size="xs"
                      variant="light"
                      color="violet"
                      onClick={() =>
                        handleTransition(engagement.id, "scheduled")
                      }
                    >
                      Schedule
                    </Button>
                  </Group>
                )}

                {/* Transition buttons for scheduled */}
                {engagement.status === "scheduled" && (
                  <Group gap="xs" mt={4}>
                    <Button
                      size="xs"
                      variant="light"
                      color="green"
                      onClick={() => handleTransition(engagement.id, "active")}
                    >
                      Start
                    </Button>
                    <Button
                      size="xs"
                      variant="light"
                      color="blue"
                      onClick={() =>
                        handleTransition(engagement.id, "completed")
                      }
                    >
                      Complete
                    </Button>
                  </Group>
                )}

                {/* Transition buttons for active */}
                {engagement.status === "active" && (
                  <Group gap="xs" mt={4}>
                    <Button
                      size="xs"
                      variant="light"
                      color="orange"
                      onClick={() => handleTransition(engagement.id, "paused")}
                    >
                      Pause
                    </Button>
                    <Button
                      size="xs"
                      variant="light"
                      color="blue"
                      onClick={() =>
                        handleTransition(engagement.id, "completed")
                      }
                    >
                      Complete
                    </Button>
                  </Group>
                )}

                {/* Transition buttons for paused */}
                {engagement.status === "paused" && (
                  <Group gap="xs" mt={4}>
                    <Button
                      size="xs"
                      variant="light"
                      color="green"
                      onClick={() => handleTransition(engagement.id, "active")}
                    >
                      Resume
                    </Button>
                    <Button
                      size="xs"
                      variant="light"
                      color="blue"
                      onClick={() =>
                        handleTransition(engagement.id, "completed")
                      }
                    >
                      Complete
                    </Button>
                  </Group>
                )}

                {engagement.tags.length > 0 && (
                  <TagGroup tags={engagement.tags} size="xs" />
                )}
              </div>
              <Group gap={4}>
                <ActionIcon
                  variant="light"
                  onClick={() => onEdit(engagement)}
                  title="Edit"
                >
                  <IconEdit size={16} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  color="red"
                  onClick={() => setArchiveTarget(engagement)}
                  title="Archive"
                >
                  <IconTrash size={16} />
                </ActionIcon>
              </Group>
            </Group>
          </Paper>
        ))}
      </Stack>

      <Modal
        opened={!!archiveTarget}
        onClose={() => setArchiveTarget(null)}
        title="Archive engagement"
        centered
      >
        <Stack>
          <Text>
            Are you sure you want to archive{" "}
            <strong>{archiveTarget?.name}</strong>? This action can be undone
            later.
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setArchiveTarget(null)}>
              Cancel
            </Button>
            <Button color="red" onClick={handleArchive}>
              Archive
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
