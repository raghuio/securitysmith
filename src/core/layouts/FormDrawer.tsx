import { Drawer, Stack, Group, Button, Alert } from "@mantine/core";
import { IconAlertCircle } from "@tabler/icons-react";
import { ReactNode } from "react";

interface FormDrawerProps {
  opened: boolean;
  onClose: () => void;
  title: string;
  error?: string | null;
  loading?: boolean;
  children: ReactNode;
  onSubmit: () => void;
  submitLabel?: string;
}

export function FormDrawer({
  opened,
  onClose,
  title,
  error,
  loading,
  children,
  onSubmit,
  submitLabel = "Save",
}: FormDrawerProps) {
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={title}
      position="right"
      size="md"
    >
      <Stack gap="md">
        {error && (
          <Alert
            icon={<IconAlertCircle size={16} />}
            color="red"
            variant="light"
          >
            {error}
          </Alert>
        )}
        {children}
        <Group justify="flex-end" gap="xs" mt="auto">
          <Button variant="default" onClick={onClose} disabled={loading}>
            Cancel
          </Button>
          <Button onClick={onSubmit} loading={loading}>
            {submitLabel}
          </Button>
        </Group>
      </Stack>
    </Drawer>
  );
}
