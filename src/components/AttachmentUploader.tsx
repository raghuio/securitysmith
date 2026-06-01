import { useState, useCallback } from "react";
import { Button, Group, Stack, Text, FileButton } from "@mantine/core";
import { IconUpload } from "@tabler/icons-react";
import { uploadAttachment } from "../api/attachments";

export function AttachmentUploader({
  entityType,
  entityId,
  onUploaded,
}: {
  entityType: string;
  entityId: number;
  onUploaded: () => void;
}) {
  const [uploading, setUploading] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  const handleFile = useCallback(
    async (file: File | null) => {
      if (!file) return;
      if (file.size > 50 * 1024 * 1024) {
        alert("File exceeds 50MB limit.");
        return;
      }
      setUploading(true);
      try {
        const reader = new FileReader();
        reader.onload = async () => {
          const base64 = (reader.result as string).split(",")[1];
          await uploadAttachment({
            entity_type: entityType,
            entity_id: entityId,
            filename: file.name,
            original_name: file.name,
            mime_type: file.type || "application/octet-stream",
            file_data_base64: base64,
          });
          onUploaded();
          setUploading(false);
        };
        reader.readAsDataURL(file);
      } catch (e) {
        console.error(e);
        setUploading(false);
      }
    },
    [entityType, entityId, onUploaded],
  );

  const handlePaste = useCallback(
    async (e: React.ClipboardEvent) => {
      const items = e.clipboardData.items;
      for (let i = 0; i < items.length; i++) {
        if (items[i].type.indexOf("image") !== -1) {
          const file = items[i].getAsFile();
          if (file) await handleFile(file);
        }
      }
    },
    [handleFile],
  );

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const files = e.dataTransfer.files;
      for (let i = 0; i < files.length; i++) {
        await handleFile(files[i]);
      }
    },
    [handleFile],
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
  }, []);

  return (
    <Stack
      gap="sm"
      onPaste={handlePaste}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      style={{
        border: dragOver
          ? "2px dashed var(--mantine-color-blue-filled)"
          : "2px dashed transparent",
        borderRadius: 8,
        padding: dragOver ? 12 : 8,
        transition: "all 0.2s ease",
        background: dragOver
          ? "var(--mantine-color-blue-light)"
          : "transparent",
      }}
    >
      <Text size="sm" c="dimmed">
        Drag & drop, click upload, or paste (Ctrl+V) images
      </Text>
      <Group>
        <FileButton
          onChange={handleFile}
          accept="image/*,application/pdf,text/*,application/json,application/xml,.har,.csv,.zip"
        >
          {(props) => (
            <Button
              leftSection={<IconUpload size={16} />}
              loading={uploading}
              {...props}
            >
              Upload
            </Button>
          )}
        </FileButton>
      </Group>
    </Stack>
  );
}
