import { useEffect, useState } from "react";
import { Button, Card, Group, Stack, Text, Badge } from "@mantine/core";
import { IconStar } from "@tabler/icons-react";
import { listContacts, deleteContact, type Contact } from "../api/contacts";
import { ContactForm } from "./ContactForm";

export function ContactList({ clientId }: { clientId: number }) {
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [editing, setEditing] = useState<Contact | null>(null);
  const [creating, setCreating] = useState(false);

  const load = async () => {
    try {
      const data = await listContacts(clientId);
      setContacts(data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [clientId]);

  const handleDelete = async (id: number) => {
    if (!confirm("Delete this contact?")) return;
    try {
      await deleteContact(id);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Text fw={600}>Contacts ({contacts.length})</Text>
        <Button size="xs" onClick={() => setCreating(true)}>
          + Add
        </Button>
      </Group>

      {creating && (
        <ContactForm
          clientId={clientId}
          onSaved={() => {
            setCreating(false);
            load();
          }}
          onCancel={() => setCreating(false)}
        />
      )}

      {editing && (
        <ContactForm
          clientId={clientId}
          contact={editing}
          onSaved={() => {
            setEditing(null);
            load();
          }}
          onCancel={() => setEditing(null)}
        />
      )}

      <Stack gap="sm">
        {contacts.map((c) => (
          <Card key={c.id} withBorder padding="sm" radius="md">
            <Group justify="space-between">
              <Group gap="sm">
                {c.is_primary && <IconStar size={16} color="gold" />}
                <Stack gap={0}>
                  <Text size="sm" fw={600}>
                    {c.name}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {c.email}
                    {c.phone && ` · ${c.phone}`}
                  </Text>
                  <Group gap="xs">
                    <Badge size="xs" variant="light">
                      {c.role.replace("_", " ")}
                    </Badge>
                    {c.title && (
                      <Badge size="xs" variant="light" color="gray">
                        {c.title}
                      </Badge>
                    )}
                  </Group>
                </Stack>
              </Group>
              <Group gap="xs">
                <Button
                  size="xs"
                  variant="subtle"
                  onClick={() => setEditing(c)}
                >
                  Edit
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  color="red"
                  onClick={() => handleDelete(c.id)}
                >
                  Delete
                </Button>
              </Group>
            </Group>
          </Card>
        ))}
      </Stack>
    </Stack>
  );
}
