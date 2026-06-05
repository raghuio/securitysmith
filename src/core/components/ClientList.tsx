import { useState, useEffect, useCallback } from "react";
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
  Badge,
} from "@mantine/core";
import { TagGroup } from "./shared";
import {
  IconArrowLeft,
  IconEdit,
  IconFolder,
  IconSearch,
  IconTrash,
  IconPlus,
} from "@tabler/icons-react";
import { listClients, deleteClient } from "../api/clients";
import type { Client } from "../api/clients";
import { listEngagements } from "../api/engagements";

interface Props {
  onBack: () => void;
  onEdit: (client: Client) => void;
  onDeleted: () => void;
  onViewEngagements: (clientId: number) => void;
  onAddEngagement: (clientId: number) => void;
  refreshKey: number;
}

export function ClientList({
  onBack,
  onEdit,
  onDeleted,
  onViewEngagements,
  onAddEngagement,
  refreshKey,
}: Props) {
  const [clients, setClients] = useState<Client[]>([]);
  const [engagementCounts, setEngagementCounts] = useState<
    Record<number, number>
  >({});
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Client | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listClients(search.trim() || undefined);
      setClients(data);
      const ids = data.map((c) => c.id);
      const counts: Record<number, number> = {};
      await Promise.all(
        ids.map(async (id) => {
          try {
            const list = await listEngagements({ clientId: id });
            counts[id] = list.length;
          } catch {
            counts[id] = 0;
          }
        }),
      );
      setEngagementCounts(counts);
    } catch (e) {
      console.error("Failed to load clients:", e);
    } finally {
      setLoading(false);
    }
  }, [search]);

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, refreshKey]);

  const handleDelete = async () => {
    if (!deleteTarget) return;
    try {
      await deleteClient(deleteTarget.id);
      setDeleteTarget(null);
      onDeleted();
      load();
    } catch (e) {
      console.error("Failed to delete client:", e);
    }
  };

  return (
    <Stack gap="md" p="md">
      <Group justify="space-between" align="center">
        <Group gap="sm">
          <ActionIcon variant="light" onClick={onBack}>
            <IconArrowLeft size={18} />
          </ActionIcon>
          <Title order={4}>Clients</Title>
        </Group>
        <Text size="sm" c="dimmed">
          {clients.length} {clients.length === 1 ? "client" : "clients"}
        </Text>
      </Group>

      <TextInput
        placeholder="Search clients..."
        leftSection={<IconSearch size={16} />}
        value={search}
        onChange={(e) => setSearch(e.currentTarget.value)}
        disabled={loading}
      />

      <Stack gap="xs">
        {clients.length === 0 && !loading && (
          <Text c="dimmed" ta="center" py="xl">
            {search.trim()
              ? "No clients match your search."
              : "No clients yet."}
          </Text>
        )}
        {clients.map((client) => (
          <Paper key={client.id} withBorder shadow="xs" p="sm" radius="md">
            <Group justify="space-between" align="flex-start">
              <div style={{ flex: 1 }}>
                <Group gap="sm">
                  <Text fw={600}>{client.name}</Text>
                  {engagementCounts[client.id] > 0 && (
                    <Badge
                      size="sm"
                      variant="light"
                      leftSection={<IconFolder size={10} />}
                      style={{ cursor: "pointer" }}
                      onClick={() => onViewEngagements(client.id)}
                    >
                      {engagementCounts[client.id]}{" "}
                      {engagementCounts[client.id] === 1
                        ? "engagement"
                        : "engagements"}
                    </Badge>
                  )}
                </Group>
                {client.contact_email && (
                  <Text size="sm" c="dimmed">
                    {client.contact_email}
                  </Text>
                )}
                {client.notes && (
                  <Text size="sm" c="dimmed" lineClamp={2}>
                    {client.notes}
                  </Text>
                )}
                {client.tags.length > 0 && (
                  <TagGroup tags={client.tags} size="xs" />
                )}
              </div>
              <Group gap={4}>
                <ActionIcon
                  variant="light"
                  onClick={() => onAddEngagement(client.id)}
                  title="Add engagement"
                >
                  <IconPlus size={16} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  onClick={() => onEdit(client)}
                  title="Edit"
                >
                  <IconEdit size={16} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  color="red"
                  onClick={() => setDeleteTarget(client)}
                  title="Delete"
                >
                  <IconTrash size={16} />
                </ActionIcon>
              </Group>
            </Group>
          </Paper>
        ))}
      </Stack>

      <Modal
        opened={!!deleteTarget}
        onClose={() => setDeleteTarget(null)}
        title="Delete client"
        centered
      >
        <Stack>
          <Text>
            Are you sure you want to delete{" "}
            <strong>{deleteTarget?.name}</strong>? This action can be undone
            later.
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button color="red" onClick={handleDelete}>
              Delete
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
