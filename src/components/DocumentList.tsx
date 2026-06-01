import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Stack,
  Text,
  Title,
  ActionIcon,
  Tooltip,
} from "@mantine/core";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import {
  listDocuments,
  archiveDocument,
  type Document,
} from "../api/documents";
import { DocumentBuilder } from "./DocumentBuilder";

export function DocumentList() {
  const [docs, setDocs] = useState<Document[]>([]);
  const [builderOpen, setBuilderOpen] = useState(false);
  const [editing, setEditing] = useState<Document | null>(null);

  const load = async () => {
    try {
      const data = await listDocuments();
      setDocs(data);
    } catch (e) {
      console.error(e);
    }
  };
  useEffect(() => {
    load();
  }, []);

  const handleEdit = (d: Document) => {
    setEditing(d);
    setBuilderOpen(true);
  };

  const handleCreate = () => {
    setEditing(null);
    setBuilderOpen(true);
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Documents</Title>
        <Group>
          <Button onClick={load}>Refresh</Button>
          <Button leftSection={<IconPlus size={16} />} onClick={handleCreate}>
            New Document
          </Button>
        </Group>
      </Group>
      {docs.map((d) => (
        <Group key={d.id} justify="space-between">
          <Text>
            {d.name} · {d.client_name} · {d.status}
          </Text>
          <Group gap="xs">
            <Tooltip label="Edit">
              <ActionIcon variant="light" onClick={() => handleEdit(d)}>
                <IconEdit size={16} />
              </ActionIcon>
            </Tooltip>
            <Button
              variant="default"
              size="xs"
              onClick={() => archiveDocument(d.id).then(load)}
            >
              Archive
            </Button>
          </Group>
        </Group>
      ))}
      {docs.length === 0 && <Text c="dimmed">No documents yet.</Text>}

      <DocumentBuilder
        opened={builderOpen}
        onClose={() => setBuilderOpen(false)}
        document={editing}
        onSaved={load}
      />
    </Stack>
  );
}
