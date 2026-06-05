import { useState, useEffect } from "react";
import {
  Alert,
  Button,
  Drawer,
  Group,
  Select,
  Stack,
  Textarea,
  TextInput,
} from "@mantine/core";
import { sendEmail } from "../api";
import { listTemplates, getTemplate } from "../../templates/api";
import type { TemplateSummary } from "../../templates/api";

interface Props {
  opened: boolean;
  onClose: () => void;
  defaultTo?: string;
  defaultSubject?: string;
  defaultBody?: string;
  clientId?: number;
  engagementId?: number;
}

export function EmailComposer({
  opened,
  onClose,
  defaultTo = "",
  defaultSubject = "",
  defaultBody = "",
  clientId,
  engagementId,
}: Props) {
  const [to, setTo] = useState(defaultTo);
  const [subject, setSubject] = useState(defaultSubject);
  const [body, setBody] = useState(defaultBody);
  const [templates, setTemplates] = useState<TemplateSummary[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (opened) {
      listTemplates("email", undefined, undefined)
        .then(setTemplates)
        .catch(console.error);
    }
  }, [opened]);

  const applyTemplate = async (templateId: string) => {
    if (!templateId) return;
    try {
      const t = await getTemplate(Number(templateId));
      setBody(t.content);
    } catch (e) {
      console.error("Failed to apply template:", e);
    }
  };

  const handleSend = async () => {
    if (!to.trim() || !subject.trim()) {
      setError("To and Subject are required.");
      return;
    }
    setError(null);
    setLoading(true);
    try {
      await sendEmail(
        to.trim(),
        subject.trim(),
        body,
        [],
        clientId,
        engagementId,
      );
      setStatus("Email sent.");
      setTo("");
      setSubject("");
      setBody("");
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
      title="Compose Email"
      position="right"
      size="xl"
    >
      <Stack gap="md">
        {templates.length > 0 && (
          <Select
            label="Apply Template (optional)"
            placeholder="Choose an email template..."
            value={selectedTemplateId}
            onChange={(val) => {
              setSelectedTemplateId(val || "");
              if (val) applyTemplate(val);
            }}
            data={templates.map((t) => ({
              value: String(t.id),
              label: t.name,
            }))}
            clearable
          />
        )}
        <TextInput
          label="To"
          placeholder="recipient@example.com"
          value={to}
          onChange={(e) => setTo(e.currentTarget.value)}
        />
        <TextInput
          label="Subject"
          placeholder="Email subject..."
          value={subject}
          onChange={(e) => setSubject(e.currentTarget.value)}
        />
        <Textarea
          label="Body"
          placeholder="Write your message..."
          value={body}
          onChange={(e) => setBody(e.currentTarget.value)}
          autosize
          minRows={10}
          maxRows={30}
        />
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}
        {status && (
          <Alert color="green" variant="light">
            {status}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSend} loading={loading}>
            Send
          </Button>
        </Group>
      </Stack>
    </Drawer>
  );
}
