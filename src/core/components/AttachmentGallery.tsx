import { useEffect, useState } from "react";
import {
  ActionIcon,
  Group,
  Image,
  Modal,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import { IconTrash, IconEye, IconPhoto } from "@tabler/icons-react";
import {
  listAttachments,
  deleteAttachment,
  renameAttachment,
  readAttachmentFile,
  getAttachmentThumbnail,
  type Attachment,
} from "../api/attachments";

export function AttachmentGallery({
  entityType,
  entityId,
}: {
  entityType: string;
  entityId: number;
}) {
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [thumbnails, setThumbnails] = useState<Record<number, string>>({});
  const [preview, setPreview] = useState<Attachment | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<number | null>(null);
  const [newName, setNewName] = useState("");

  const loadThumbnails = async (data: Attachment[]) => {
    const urls: Record<number, string> = {};
    await Promise.all(
      data.map(async (att) => {
        if (isImage(att.mime_type)) {
          try {
            const base64 = await getAttachmentThumbnail(
              entityType,
              entityId,
              att.filename,
            );
            urls[att.id] = `data:image/png;base64,${base64}`;
          } catch (e) {
            console.error(e);
          }
        }
      }),
    );
    setThumbnails(urls);
  };

  const load = async () => {
    try {
      const data = await listAttachments(entityType, entityId);
      setAttachments(data);
      await loadThumbnails(data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [entityType, entityId]);

  const handleDelete = async (id: number) => {
    if (!confirm("Delete this attachment?")) return;
    try {
      await deleteAttachment(id);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleRename = async (id: number) => {
    try {
      await renameAttachment(id, newName);
      setRenaming(null);
      setNewName("");
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const openPreview = async (att: Attachment) => {
    try {
      const base64 = await readAttachmentFile(
        entityType,
        entityId,
        att.filename,
      );
      const url = `data:${att.mime_type};base64,${base64}`;
      setPreviewUrl(url);
      setPreview(att);
    } catch (e) {
      console.error(e);
    }
  };

  const isImage = (mime: string) => mime.startsWith("image/");

  return (
    <Stack gap="sm">
      {attachments.map((att) => (
        <Group key={att.id} justify="space-between">
          <Group gap="sm">
            {isImage(att.mime_type) && thumbnails[att.id] && (
              <Image
                src={thumbnails[att.id]}
                alt={att.original_name}
                width={40}
                height={40}
                fit="cover"
                radius="sm"
                style={{ cursor: "pointer" }}
                onClick={() => openPreview(att)}
              />
            )}
            {!thumbnails[att.id] && isImage(att.mime_type) && (
              <IconPhoto size={40} stroke={1.5} />
            )}
            <Stack gap={0}>
              {renaming === att.id ? (
                <TextInput
                  size="xs"
                  value={newName}
                  onChange={(e) => setNewName(e.currentTarget.value)}
                  onBlur={() => handleRename(att.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleRename(att.id);
                  }}
                  autoFocus
                />
              ) : (
                <Text
                  size="sm"
                  style={{ cursor: "pointer" }}
                  onClick={() => {
                    setRenaming(att.id);
                    setNewName(att.original_name);
                  }}
                >
                  {att.original_name}
                </Text>
              )}
              <Text size="xs" c="dimmed">
                {(att.file_size / 1024).toFixed(1)} KB · {att.mime_type}
              </Text>
            </Stack>
          </Group>
          <Group gap="xs">
            {isImage(att.mime_type) && (
              <ActionIcon variant="light" onClick={() => openPreview(att)}>
                <IconEye size={16} />
              </ActionIcon>
            )}
            <ActionIcon
              variant="light"
              color="red"
              onClick={() => handleDelete(att.id)}
            >
              <IconTrash size={16} />
            </ActionIcon>
          </Group>
        </Group>
      ))}

      {attachments.length === 0 && (
        <Text c="dimmed" size="sm">
          No attachments
        </Text>
      )}

      <Modal
        opened={!!preview}
        onClose={() => setPreview(null)}
        title={preview?.original_name}
        size="xl"
      >
        {previewUrl && isImage(preview?.mime_type || "") && (
          <Image src={previewUrl} alt={preview?.original_name} fit="contain" />
        )}
      </Modal>
    </Stack>
  );
}
