import { useState, useEffect } from "react";
import {
  ActionIcon,
  Badge,
  Button,
  Group,
  Paper,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { IconEdit, IconTrash, IconPlus } from "@tabler/icons-react";
import { CredentialForm } from "./CredentialForm";
import { listCredentials, deleteCredential } from "../api/credentials";
import type { Credential, CredentialStatus } from "../api/credentials";
import { updateCredential } from "../api/credentials";

interface Props {
  engagementId: number;
  refreshKey: number;
}

const STATUS_COLORS: Record<CredentialStatus, string> = {
  not_verified: "gray",
  working: "green",
  not_working: "red",
  expired: "orange",
};

export function CredentialList({ engagementId, refreshKey }: Props) {
  const [credentials, setCredentials] = useState<Credential[]>([]);
  const [loading, setLoading] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Credential | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await listCredentials(engagementId);
      setCredentials(data);
    } catch (e) {
      console.error("Failed to load credentials:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [engagementId, refreshKey]);

  const handleStatusChange = async (
    cred: Credential,
    status: CredentialStatus,
  ) => {
    try {
      await updateCredential(cred.id, { status });
      load();
    } catch (e) {
      console.error("Failed to update credential status:", e);
    }
  };

  const handleDelete = async (cred: Credential) => {
    if (
      !window.confirm(
        `Delete credential "${cred.label}"? This cannot be undone.`,
      )
    )
      return;
    try {
      await deleteCredential(cred.id);
      load();
    } catch (e) {
      console.error("Failed to delete credential:", e);
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between" align="center">
        <Title order={5}>Credentials</Title>
        <Button
          leftSection={<IconPlus size={14} />}
          variant="light"
          size="xs"
          onClick={() => {
            setEditing(null);
            setFormOpen(true);
          }}
        >
          Add credential
        </Button>
      </Group>

      {credentials.length === 0 && !loading && (
        <Text c="dimmed" size="sm">
          No credentials yet.
        </Text>
      )}

      <Stack gap="xs">
        {credentials.map((cred) => (
          <Paper key={cred.id} withBorder p="sm" radius="sm">
            <Group justify="space-between" align="flex-start">
              <div style={{ flex: 1 }}>
                <Group gap="sm">
                  <Text fw={600} size="sm">
                    {cred.label}
                  </Text>
                  <Badge color={STATUS_COLORS[cred.status]} size="xs">
                    {cred.status}
                  </Badge>
                </Group>
                <Text size="xs" c="dimmed">
                  {cred.credential_type}
                </Text>
                {cred.notes && (
                  <Text size="xs" c="dimmed" lineClamp={1}>
                    {cred.notes}
                  </Text>
                )}
                <Group gap={4} mt={4}>
                  <Button
                    size="xs"
                    variant={cred.status === "working" ? "filled" : "light"}
                    color="green"
                    onClick={() => handleStatusChange(cred, "working")}
                  >
                    Working
                  </Button>
                  <Button
                    size="xs"
                    variant={cred.status === "expired" ? "filled" : "light"}
                    color="orange"
                    onClick={() => handleStatusChange(cred, "expired")}
                  >
                    Expired
                  </Button>
                  <Button
                    size="xs"
                    variant={cred.status === "not_working" ? "filled" : "light"}
                    color="red"
                    onClick={() => handleStatusChange(cred, "not_working")}
                  >
                    Not Working
                  </Button>
                </Group>
              </div>
              <Group gap={4}>
                <ActionIcon
                  variant="light"
                  size="sm"
                  onClick={() => {
                    setEditing(cred);
                    setFormOpen(true);
                  }}
                  title="Edit"
                >
                  <IconEdit size={14} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  size="sm"
                  color="red"
                  onClick={() => handleDelete(cred)}
                  title="Delete"
                >
                  <IconTrash size={14} />
                </ActionIcon>
              </Group>
            </Group>
          </Paper>
        ))}
      </Stack>

      {formOpen && (
        <CredentialForm
          opened={formOpen}
          credential={editing}
          engagementId={engagementId}
          onClose={() => {
            setFormOpen(false);
            setEditing(null);
          }}
          onSaved={() => {
            setFormOpen(false);
            setEditing(null);
            load();
          }}
        />
      )}
    </Stack>
  );
}
