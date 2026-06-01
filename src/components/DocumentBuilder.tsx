import { useEffect, useState, useMemo } from "react";
import {
  Alert,
  Button,
  Drawer,
  Group,
  Select,
  Stack,
  Textarea,
  TextInput,
  Paper,
  ScrollArea,
  Switch,
  Text,
} from "@mantine/core";
import { marked } from "marked";
import {
  createDocument,
  updateDocument,
  renderDocumentPlaceholders,
} from "../api/documents";
import { listTemplates, getTemplate } from "../api/templates";
import type { TemplateSummary } from "../api/templates";
import type { Document } from "../api/documents";
import { listClients } from "../api/clients";
import type { Client } from "../api/clients";
import { listEngagements } from "../api/engagements";
import type { Engagement } from "../api/engagements";

interface Props {
  opened: boolean;
  onClose: () => void;
  document?: Document | null;
  onSaved: () => void;
}

export function DocumentBuilder({
  opened,
  onClose,
  document: doc,
  onSaved,
}: Props) {
  const isEdit = !!doc;
  const [name, setName] = useState(doc?.name || "");
  const [documentType, setDocumentType] = useState(doc?.document_type || "SOW");
  const [content, setContent] = useState(doc?.content || "");
  const [clientId, setClientId] = useState<string>(
    doc ? String(doc.client_id) : "",
  );
  const [engagementId, setEngagementId] = useState<string>(
    doc?.engagement_id ? String(doc.engagement_id) : "",
  );
  const [clients, setClients] = useState<Client[]>([]);
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const [templates, setTemplates] = useState<TemplateSummary[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPreview, setShowPreview] = useState(true);

  const previewHtml = useMemo(() => {
    return marked.parse(content || "", { async: false }) as string;
  }, [content]);

  useEffect(() => {
    if (opened) {
      setName(doc?.name || "");
      setDocumentType(doc?.document_type || "SOW");
      setContent(doc?.content || "");
      setClientId(doc ? String(doc.client_id) : "");
      setEngagementId(doc?.engagement_id ? String(doc.engagement_id) : "");
      listClients().then(setClients).catch(console.error);
      listEngagements().then(setEngagements).catch(console.error);
      listTemplates(undefined, undefined, undefined)
        .then((all) => {
          setTemplates(
            all.filter(
              (t) =>
                t.category === "requirements" ||
                t.category === "email" ||
                t.category === "status_report" ||
                t.category === "engagement_status",
            ),
          );
        })
        .catch(console.error);
    }
  }, [opened, doc]);

  const applyTemplate = async (templateId: string) => {
    if (!templateId) return;
    try {
      const t = await getTemplate(Number(templateId));
      setContent(t.content);
    } catch (e) {
      console.error("Failed to apply template:", e);
    }
  };

  const handleSave = async () => {
    if (!name.trim() || !clientId) {
      setError("Name and Client are required.");
      return;
    }
    setError(null);
    setLoading(true);
    try {
      const cid = Number(clientId);
      const eid = engagementId ? Number(engagementId) : undefined;
      if (isEdit && doc) {
        await updateDocument(doc.id, {
          name: name.trim(),
          content,
          status: doc.status,
        });
      } else {
        await createDocument(cid, name.trim(), documentType, content, eid);
      }
      onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handlePreviewPlaceholders = async () => {
    if (!clientId) return;
    try {
      const rendered = await renderDocumentPlaceholders(
        content,
        Number(clientId),
        engagementId ? Number(engagementId) : undefined,
      );
      setContent(rendered);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={isEdit ? "Edit Document" : "New Document"}
      position="right"
      size="xl"
    >
      <Stack gap="md">
        {!isEdit && templates.length > 0 && (
          <Select
            label="Apply Template (optional)"
            placeholder="Choose a document template..."
            value={selectedTemplateId}
            onChange={(val) => {
              setSelectedTemplateId(val || "");
              if (val) applyTemplate(val);
            }}
            data={templates.map((t) => ({
              value: String(t.id),
              label: `${t.name} (${t.category})`,
            }))}
            clearable
          />
        )}

        <TextInput
          label="Name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
        />
        {!isEdit && (
          <Select
            label="Document Type"
            value={documentType}
            onChange={(v) => setDocumentType(v || "SOW")}
            data={[
              { value: "SOW", label: "Statement of Work" },
              { value: "Proposal", label: "Proposal" },
              { value: "Custom", label: "Custom" },
            ]}
          />
        )}
        <Select
          label="Client"
          value={clientId}
          onChange={(v) => setClientId(v || "")}
          data={clients.map((c) => ({
            value: String(c.id),
            label: c.name,
          }))}
          searchable
        />
        <Select
          label="Engagement (optional)"
          value={engagementId}
          onChange={(v) => setEngagementId(v || "")}
          data={[
            { value: "", label: "None" },
            ...engagements.map((e) => ({
              value: String(e.id),
              label: e.name,
            })),
          ]}
          searchable
        />

        <Group justify="space-between">
          <Switch
            label="Show live preview"
            checked={showPreview}
            onChange={(event) => setShowPreview(event.currentTarget.checked)}
          />
          <Button variant="light" size="xs" onClick={handlePreviewPlaceholders}>
            Render Placeholders
          </Button>
        </Group>

        <Group align="flex-start" gap="md" style={{ flex: 1 }}>
          <Textarea
            label="Content (Markdown)"
            value={content}
            onChange={(e) => setContent(e.currentTarget.value)}
            autosize
            minRows={14}
            maxRows={30}
            style={{ flex: 1, minWidth: 280 }}
          />
          {showPreview && (
            <Paper
              withBorder
              p="sm"
              radius="sm"
              style={{ flex: 1, minWidth: 280, minHeight: 300 }}
            >
              <Text size="sm" fw={500} mb="xs">
                Live Preview
              </Text>
              <ScrollArea h={320}>
                <div
                  className="markdown-preview"
                  style={{ fontSize: 14, lineHeight: 1.5 }}
                  dangerouslySetInnerHTML={{ __html: previewHtml }}
                />
              </ScrollArea>
            </Paper>
          )}
        </Group>

        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave} loading={loading}>
            Save
          </Button>
        </Group>
      </Stack>
    </Drawer>
  );
}
