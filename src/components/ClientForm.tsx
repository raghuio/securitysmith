import { useState, useEffect } from "react";
import {
  Button,
  Drawer,
  Stack,
  Textarea,
  TextInput,
  Text,
  Alert,
} from "@mantine/core";
import { createClient, updateClient } from "../api/clients";
import type { Client } from "../api/clients";

import { ContactList } from "./ContactList";

interface Props {
  opened: boolean;
  client: Client | null;
  onClose: () => void;
  onSaved: () => void;
}

export function ClientForm({ opened, client, onClose, onSaved }: Props) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [notes, setNotes] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [techStack, setTechStack] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isEdit = !!client;

  useEffect(() => {
    if (client) {
      setName(client.name);
      setEmail(client.contact_email ?? "");
      setNotes(client.notes ?? "");
      setTags(client.tags);
      setTechStack(client.tech_stack ?? []);
    } else {
      setName("");
      setEmail("");
      setNotes("");
      setTags([]);
      setTechStack([]);
    }
    setError(null);
  }, [client]);

  const validate = (): boolean => {
    if (!name.trim()) {
      setError("Client name is required.");
      return false;
    }
    if (name.trim().length > 255) {
      setError("Client name must be 255 characters or fewer.");
      return false;
    }
    if (email && email.length > 0) {
      const parts = email.split("@");
      if (
        parts.length !== 2 ||
        !parts[1].includes(".") ||
        parts[1].startsWith(".") ||
        parts[1].endsWith(".")
      ) {
        setError("Contact email is invalid.");
        return false;
      }
    }
    if (notes.length > 10000) {
      setError("Notes must be 10,000 characters or fewer.");
      return false;
    }
    return true;
  };

  const handleSubmit = async () => {
    setError(null);
    if (!validate()) return;

    setLoading(true);
    try {
      if (isEdit && client) {
        await updateClient(
          client.id,
          name.trim(),
          email || undefined,
          notes || undefined,
          tags.length > 0 ? tags : undefined,
          techStack.length > 0 ? techStack : undefined,
        );
      } else {
        await createClient(
          name.trim(),
          email || undefined,
          notes || undefined,
          tags.length > 0 ? tags : undefined,
          techStack.length > 0 ? techStack : undefined,
        );
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
      title={isEdit ? "Edit Client" : "New Client"}
      position="right"
      size="md"
    >
      <Stack>
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}

        <TextInput
          label="Name"
          placeholder="Acme Corp"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          required
          autoFocus={!isEdit}
        />

        <TextInput
          label="Contact Email"
          placeholder="security@acme.com"
          value={email}
          onChange={(e) => setEmail(e.currentTarget.value)}
        />

        <Textarea
          label="Notes"
          placeholder="Any notes about this client..."
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          minRows={3}
          maxRows={8}
        />

        <TextInput
          label="Tags"
          placeholder="Enter tags separated by commas..."
          value={tags.join(", ")}
          onChange={(e) => {
            const raw = e.currentTarget.value;
            const items = raw
              .split(",")
              .map((s) => s.trim().toLowerCase().replace(/\s+/g, "-"))
              .filter((s) => s.length > 0);
            setTags(items);
          }}
        />

        <TextInput
          label="Tech Stack"
          placeholder="nginx, wordpress, aws, java..."
          value={techStack.join(", ")}
          onChange={(e) => {
            const raw = e.currentTarget.value;
            const items = raw
              .split(",")
              .map((s) => s.trim().toLowerCase().replace(/\s+/g, "-"))
              .filter((s) => s.length > 0);
            setTechStack(items);
          }}
        />

        <Text size="xs" c="dimmed">
          {notes.length} / 10,000 characters
        </Text>

        <Stack gap="xs" mt="md">
          <Button onClick={handleSubmit} loading={loading}>
            {isEdit ? "Save Changes" : "Create Client"}
          </Button>
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
        </Stack>

        {isEdit && client && <ContactList clientId={client.id} />}
      </Stack>
    </Drawer>
  );
}
