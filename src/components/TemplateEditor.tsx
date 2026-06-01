import { useState } from "react";
import {
  Button,
  Group,
  Select,
  Stack,
  TextInput,
  Textarea,
} from "@mantine/core";
import type { Template } from "../api/templates";

export interface TemplateEditorValues {
  name: string;
  category: string;
  subcategory: string;
  content: string;
  tags: string;
}

interface Props {
  initial: Template | null;
  onSave: (values: TemplateEditorValues) => void;
  onCancel: () => void;
}

export function TemplateEditor({ initial, onSave, onCancel }: Props) {
  const [name, setName] = useState(initial?.name || "");
  const [category, setCategory] = useState(initial?.category || "finding");
  const [subcategory, setSubcategory] = useState(initial?.subcategory || "");
  const [content, setContent] = useState(initial?.content || "");
  const [tags, setTags] = useState(initial?.tags?.join(", ") || "");

  return (
    <Stack gap="md">
      <TextInput
        label="Name"
        value={name}
        onChange={(e) => setName(e.currentTarget.value)}
      />
      {!initial && (
        <Select
          label="Category"
          value={category}
          onChange={(v) => setCategory(v || "finding")}
          data={[
            { value: "finding", label: "Finding" },
            { value: "requirements", label: "Requirements" },
            { value: "checklist", label: "Checklist" },
            { value: "email", label: "Email" },
            { value: "status_report", label: "Status Report" },
            { value: "engagement_status", label: "Engagement Status" },
          ]}
          allowDeselect={false}
        />
      )}
      <TextInput
        label="Subcategory"
        value={subcategory}
        onChange={(e) => setSubcategory(e.currentTarget.value)}
      />
      <Textarea
        label="Content"
        value={content}
        onChange={(e) => setContent(e.currentTarget.value)}
        autosize
        minRows={10}
        maxRows={30}
      />
      <TextInput
        label="Tags (comma separated)"
        value={tags}
        onChange={(e) => setTags(e.currentTarget.value)}
      />
      <Group justify="flex-end">
        <Button variant="default" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          onClick={() => onSave({ name, category, subcategory, content, tags })}
        >
          Save
        </Button>
      </Group>
    </Stack>
  );
}
