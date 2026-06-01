import { useState } from "react";
import {
  Button,
  Checkbox,
  Group,
  Select,
  Stack,
  TextInput,
} from "@mantine/core";
import { createContact, updateContact, type Contact } from "../api/contacts";

const ROLES = [
  { value: "technical_poc", label: "Technical POC" },
  { value: "management", label: "Management" },
  { value: "billing", label: "Billing" },
  { value: "legal", label: "Legal" },
  { value: "remediation", label: "Remediation" },
  { value: "executive", label: "Executive" },
  { value: "other", label: "Other" },
];

export function ContactForm({
  clientId,
  contact,
  onSaved,
  onCancel,
}: {
  clientId: number;
  contact?: Contact | null;
  onSaved: () => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(contact?.name || "");
  const [email, setEmail] = useState(contact?.email || "");
  const [phone, setPhone] = useState(contact?.phone || "");
  const [role, setRole] = useState(contact?.role || "other");
  const [roleLabel, setRoleLabel] = useState(contact?.role_label || "");
  const [title, setTitle] = useState(contact?.title || "");
  const [notes, setNotes] = useState(contact?.notes || "");
  const [isPrimary, setIsPrimary] = useState(contact?.is_primary || false);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    setLoading(true);
    try {
      const input = {
        client_id: clientId,
        name,
        email,
        phone: phone || undefined,
        role,
        role_label: roleLabel || undefined,
        title: title || undefined,
        notes: notes || undefined,
        is_primary: isPrimary,
      };
      if (contact) {
        await updateContact(contact.id, input);
      } else {
        await createContact(input);
      }
      onSaved();
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Stack gap="sm">
      <TextInput
        label="Name"
        value={name}
        onChange={(e) => setName(e.currentTarget.value)}
        required
      />
      <TextInput
        label="Email"
        value={email}
        onChange={(e) => setEmail(e.currentTarget.value)}
        required
      />
      <TextInput
        label="Phone"
        value={phone}
        onChange={(e) => setPhone(e.currentTarget.value)}
      />
      <Select
        label="Role"
        data={ROLES}
        value={role}
        onChange={(v) => v && setRole(v)}
        required
      />
      {role === "other" && (
        <TextInput
          label="Role Label"
          value={roleLabel}
          onChange={(e) => setRoleLabel(e.currentTarget.value)}
        />
      )}
      <TextInput
        label="Title"
        value={title}
        onChange={(e) => setTitle(e.currentTarget.value)}
      />
      <TextInput
        label="Notes"
        value={notes}
        onChange={(e) => setNotes(e.currentTarget.value)}
      />
      <Checkbox
        label="Primary contact"
        checked={isPrimary}
        onChange={(e) => setIsPrimary(e.currentTarget.checked)}
      />
      <Group justify="flex-end">
        <Button variant="default" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={handleSubmit} loading={loading}>
          Save
        </Button>
      </Group>
    </Stack>
  );
}
