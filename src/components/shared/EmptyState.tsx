import { Button, Stack, Text, Title } from "@mantine/core";

interface EmptyStateProps {
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function EmptyState({
  title,
  description,
  actionLabel,
  onAction,
}: EmptyStateProps) {
  return (
    <Stack align="center" gap="sm" py="xl">
      <Title order={4} c="dimmed">
        {title}
      </Title>
      {description && <Text c="dimmed">{description}</Text>}
      {actionLabel && onAction && (
        <Button onClick={onAction}>{actionLabel}</Button>
      )}
    </Stack>
  );
}
