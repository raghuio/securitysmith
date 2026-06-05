import { useEffect, useState } from "react";
import {
  Badge,
  Button,
  Group,
  Stack,
  Text,
  Title,
  ActionIcon,
  Tooltip,
} from "@mantine/core";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { listInvoices, archiveInvoice, type Invoice } from "../api";
import { InvoiceBuilder } from "./InvoiceBuilder";

export function InvoiceList() {
  const [items, setItems] = useState<Invoice[]>([]);
  const [builderOpen, setBuilderOpen] = useState(false);
  const [editing, setEditing] = useState<Invoice | null>(null);

  const load = async () => {
    try {
      const data = await listInvoices();
      setItems(data);
    } catch (e) {
      console.error(e);
    }
  };
  useEffect(() => {
    load();
  }, []);

  const handleEdit = (i: Invoice) => {
    setEditing(i);
    setBuilderOpen(true);
  };

  const handleCreate = () => {
    setEditing(null);
    setBuilderOpen(true);
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Invoices</Title>
        <Group>
          <Button onClick={load}>Refresh</Button>
          <Button leftSection={<IconPlus size={16} />} onClick={handleCreate}>
            New Invoice
          </Button>
        </Group>
      </Group>
      {items.map((i) => (
        <Group key={i.id} justify="space-between">
          <Group>
            <Text>
              {i.invoice_number} · {i.client_name}
            </Text>
            <Badge>{i.status}</Badge>
          </Group>
          <Group gap="xs">
            <Tooltip label="Edit">
              <ActionIcon variant="light" onClick={() => handleEdit(i)}>
                <IconEdit size={16} />
              </ActionIcon>
            </Tooltip>
            <Button
              variant="default"
              size="xs"
              onClick={() => archiveInvoice(i.id).then(load)}
            >
              Archive
            </Button>
          </Group>
        </Group>
      ))}
      {items.length === 0 && <Text c="dimmed">No invoices yet.</Text>}

      <InvoiceBuilder
        opened={builderOpen}
        onClose={() => setBuilderOpen(false)}
        invoice={editing}
        onSaved={load}
      />
    </Stack>
  );
}
