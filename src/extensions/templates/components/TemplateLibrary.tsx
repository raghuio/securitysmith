import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Group,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
  Badge,
  ScrollArea,
  ActionIcon,
  Tooltip,
  Modal,
  Drawer,
  Textarea,
} from "@mantine/core";
import { IconCopy, IconTrash, IconPlus, IconSearch } from "@tabler/icons-react";
import { TemplateEditor } from "./TemplateEditor";
import type { TemplateEditorValues } from "./TemplateEditor";
import {
  listTemplates,
  duplicateTemplate,
  deleteTemplate,
  createTemplate,
  updateTemplate,
  type TemplateSummary,
  type Template,
} from "../api";
import { getTemplate } from "../api";

type FilterCategory =
  | "all"
  | "finding"
  | "requirements"
  | "checklist"
  | "email"
  | "status_report"
  | "engagement_status";

export function TemplateLibrary() {
  const [templates, setTemplates] = useState<TemplateSummary[]>([]);
  const [category, setCategory] = useState<FilterCategory>("all");
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Template | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [preview, setPreview] = useState<Template | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await listTemplates(
        category === "all" ? undefined : category,
        undefined,
        search || undefined,
      );
      setTemplates(data);
    } catch (e) {
      console.error("Failed to load templates:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [category, search]);

  const handleDuplicate = async (id: number) => {
    try {
      await duplicateTemplate(id);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteTemplate(id);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleSave = async (values: TemplateEditorValues) => {
    try {
      const tags = values.tags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);
      if (editing) {
        await updateTemplate(editing.id, {
          name: values.name,
          content: values.content,
          tags,
        });
      } else {
        await createTemplate({
          name: values.name,
          category:
            values.category as import("../../templates/api").TemplateCategory,
          subcategory: values.subcategory,
          content: values.content,
          tags,
        });
      }
      setEditorOpen(false);
      setEditing(null);
      load();
    } catch (e) {
      console.error(e);
    }
  };

  const openEdit = async (id: number) => {
    try {
      const t = await getTemplate(id);
      setEditing(t);
      setEditorOpen(true);
    } catch (e) {
      console.error(e);
    }
  };

  const openPreview = async (id: number) => {
    try {
      const t = await getTemplate(id);
      setPreview(t);
      setPreviewOpen(true);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Templates</Title>
        <Button
          leftSection={<IconPlus size={16} />}
          onClick={() => {
            setEditing(null);
            setEditorOpen(true);
          }}
        >
          New Template
        </Button>
      </Group>

      <Group>
        <Select
          label="Category"
          value={category}
          onChange={(v) => setCategory(v as FilterCategory)}
          data={[
            { value: "all", label: "All" },
            { value: "finding", label: "Findings" },
            { value: "requirements", label: "Requirements" },
            { value: "checklist", label: "Checklists" },
            { value: "email", label: "Emails" },
            { value: "status_report", label: "Status Reports" },
            { value: "engagement_status", label: "Engagement Status" },
          ]}
          style={{ width: 200 }}
          allowDeselect={false}
        />
        <TextInput
          label="Search"
          placeholder="Search templates..."
          value={search}
          onChange={(e) => setSearch(e.currentTarget.value)}
          leftSection={<IconSearch size={16} />}
          style={{ flex: 1 }}
        />
      </Group>

      <ScrollArea style={{ height: "calc(100vh - 260px)" }}>
        <Stack gap="sm">
          {templates.map((t) => (
            <Card key={t.id} withBorder>
              <Group justify="space-between" wrap="nowrap">
                <Stack
                  gap={2}
                  style={{ flex: 1, cursor: "pointer" }}
                  onClick={() => openPreview(t.id)}
                >
                  <Group gap="xs">
                    <Text fw={600}>{t.name}</Text>
                    {t.is_builtin && (
                      <Badge color="blue" variant="light">
                        Built-in
                      </Badge>
                    )}
                  </Group>
                  <Text size="xs" c="dimmed">
                    {t.category} · {t.subcategory || "—"}
                  </Text>
                  <Group gap="xs">
                    {t.tags.slice(0, 4).map((tag) => (
                      <Badge key={tag} size="xs" variant="outline">
                        {tag}
                      </Badge>
                    ))}
                  </Group>
                </Stack>
                <Group gap="xs">
                  <Tooltip label="Duplicate">
                    <ActionIcon
                      variant="light"
                      onClick={() => handleDuplicate(t.id)}
                    >
                      <IconCopy size={16} />
                    </ActionIcon>
                  </Tooltip>
                  {!t.is_builtin && (
                    <>
                      <Tooltip label="Edit">
                        <ActionIcon
                          variant="light"
                          onClick={() => openEdit(t.id)}
                        >
                          <IconPlus size={16} />
                        </ActionIcon>
                      </Tooltip>
                      <Tooltip label="Delete">
                        <ActionIcon
                          variant="light"
                          color="red"
                          onClick={() => handleDelete(t.id)}
                        >
                          <IconTrash size={16} />
                        </ActionIcon>
                      </Tooltip>
                    </>
                  )}
                </Group>
              </Group>
            </Card>
          ))}
          {templates.length === 0 && !loading && (
            <Text c="dimmed" ta="center">
              No templates found.
            </Text>
          )}
        </Stack>
      </ScrollArea>

      <Drawer
        opened={editorOpen}
        onClose={() => setEditorOpen(false)}
        title={editing ? "Edit Template" : "New Template"}
        position="right"
        size="xl"
      >
        <TemplateEditor
          initial={editing}
          onSave={handleSave}
          onCancel={() => setEditorOpen(false)}
        />
      </Drawer>

      <Modal
        opened={previewOpen}
        onClose={() => setPreviewOpen(false)}
        title={preview?.name || "Template"}
        size="lg"
      >
        {preview && (
          <Stack>
            <Group>
              <Badge>{preview.category}</Badge>
              <Badge variant="outline">{preview.subcategory}</Badge>
              {preview.is_builtin && <Badge color="blue">Built-in</Badge>}
            </Group>
            <Textarea
              value={preview.content}
              readOnly
              autosize
              minRows={10}
              maxRows={30}
              style={{ fontFamily: "monospace" }}
            />
          </Stack>
        )}
      </Modal>
    </Stack>
  );
}
