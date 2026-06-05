import { Card, Group, Stack, Text, ActionIcon } from "@mantine/core";
import { IconEdit, IconArchive } from "@tabler/icons-react";
import { TagGroup } from "./TagGroup";
import { StatusBadge } from "./StatusBadge";

interface EntityCardProps {
  title: string;
  subtitle?: string;
  metadata?: string;
  tags?: string[];
  status?: string;
  statusType?: "engagement" | "finding" | "invoice" | "document";
  onEdit?: () => void;
  onArchive?: () => void;
}

export function EntityCard({
  title,
  subtitle,
  metadata,
  tags,
  status,
  statusType,
  onEdit,
  onArchive,
}: EntityCardProps) {
  return (
    <Card withBorder shadow="sm" padding="sm" radius="md">
      <Stack gap="xs">
        <Group justify="space-between" wrap="nowrap">
          <Stack gap={0}>
            <Text fw={600} size="sm" lineClamp={1}>
              {title}
            </Text>
            {subtitle && (
              <Text size="xs" c="dimmed" lineClamp={1}>
                {subtitle}
              </Text>
            )}
          </Stack>
          <Group gap="xs">
            {onEdit && (
              <ActionIcon variant="subtle" size="sm" onClick={onEdit}>
                <IconEdit size={14} />
              </ActionIcon>
            )}
            {onArchive && (
              <ActionIcon
                variant="subtle"
                size="sm"
                color="red"
                onClick={onArchive}
              >
                <IconArchive size={14} />
              </ActionIcon>
            )}
          </Group>
        </Group>
        {metadata && (
          <Text size="xs" c="dimmed">
            {metadata}
          </Text>
        )}
        <Group justify="space-between">
          <TagGroup tags={tags || []} size="xs" />
          {status && statusType && (
            <StatusBadge value={status} type={statusType} size="xs" />
          )}
        </Group>
      </Stack>
    </Card>
  );
}
