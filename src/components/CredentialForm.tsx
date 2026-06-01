import { useState, useEffect } from "react";
import {
  Button,
  Drawer,
  Stack,
  Textarea,
  TextInput,
  Text,
  Alert,
  Select,
} from "@mantine/core";
import { createCredential, updateCredential } from "../api/credentials";
import type { Credential, CredentialInput } from "../api/credentials";

interface Props {
  opened: boolean;
  credential: Credential | null;
  engagementId: number;
  onClose: () => void;
  onSaved: () => void;
}

const TYPE_OPTIONS = [
  "url",
  "username_password",
  "api_key",
  "bearer_token",
  "vpn_config",
  "ssh_key",
  "custom",
];

export function CredentialForm({
  opened,
  credential,
  engagementId,
  onClose,
  onSaved,
}: Props) {
  const isEdit = !!credential;
  const [label, setLabel] = useState("");
  const [credentialType, setCredentialType] = useState("username_password");
  const [value, setValue] = useState("");
  const [notes, setNotes] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (credential) {
      setLabel(credential.label);
      setCredentialType(credential.credential_type);
      setValue(credential.value);
      setNotes(credential.notes ?? "");
    } else {
      setLabel("");
      setCredentialType("username_password");
      setValue("");
      setNotes("");
    }
    setError(null);
  }, [credential]);

  const validate = (): boolean => {
    if (!label.trim()) {
      setError("Label is required.");
      return false;
    }
    if (value.length === 0) {
      setError("Credential value is required.");
      return false;
    }
    if (value.length > 50_000) {
      setError("Credential value exceeds maximum size of 50KB.");
      return false;
    }
    if (notes.length > 5_000) {
      setError("Notes must be 5,000 characters or fewer.");
      return false;
    }
    return true;
  };

  const handleSubmit = async () => {
    setError(null);
    if (!validate()) return;

    setLoading(true);
    try {
      const input: CredentialInput = {
        engagement_id: engagementId,
        label: label.trim(),
        credential_type: credentialType.trim(),
        value,
        notes: notes || undefined,
      };

      if (isEdit && credential) {
        await updateCredential(credential.id, {
          label: input.label,
          credential_type: input.credential_type,
          value: input.value,
          notes: input.notes,
        });
      } else {
        await createCredential(input);
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
      title={isEdit ? "Edit Credential" : "New Credential"}
      position="right"
      size="sm"
    >
      <Stack>
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}

        <TextInput
          label="Label"
          placeholder="Admin login"
          value={label}
          onChange={(e) => setLabel(e.currentTarget.value)}
          required
          autoFocus={!isEdit}
        />

        <Select
          label="Type"
          data={TYPE_OPTIONS}
          value={credentialType}
          onChange={(val) => setCredentialType(val ?? "custom")}
          required
        />

        <Textarea
          label="Value"
          placeholder="Enter credential value..."
          value={value}
          onChange={(e) => setValue(e.currentTarget.value)}
          minRows={3}
          maxRows={12}
          required
        />

        <Textarea
          label="Notes"
          placeholder="Optional notes..."
          value={notes}
          onChange={(e) => setNotes(e.currentTarget.value)}
          minRows={2}
          maxRows={6}
        />

        <Text size="xs" c="dimmed">
          {notes.length} / 5,000 characters
        </Text>

        <Stack gap="xs" mt="md">
          <Button onClick={handleSubmit} loading={loading}>
            {isEdit ? "Save Changes" : "Create Credential"}
          </Button>
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
        </Stack>
      </Stack>
    </Drawer>
  );
}
