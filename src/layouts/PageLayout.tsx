import { Group, Stack, Text, Title } from "@mantine/core";
import { ReactNode } from "react";

interface PageLayoutProps {
  title: string;
  breadcrumbs?: string[];
  actions?: ReactNode;
  children: ReactNode;
}

export function PageLayout({
  title,
  breadcrumbs,
  actions,
  children,
}: PageLayoutProps) {
  return (
    <Stack gap="md" p="md" style={{ maxWidth: 1200, margin: "0 auto" }}>
      {breadcrumbs && breadcrumbs.length > 0 && (
        <Text size="xs" c="dimmed">
          {breadcrumbs.join(" / ")}
        </Text>
      )}
      <Group justify="space-between" wrap="nowrap">
        <Title order={3}>{title}</Title>
        {actions && <Group gap="xs">{actions}</Group>}
      </Group>
      {children}
    </Stack>
  );
}
