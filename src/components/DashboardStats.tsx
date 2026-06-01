import { SimpleGrid, Card, Text, Group } from "@mantine/core";

interface StatCardProps {
  title: string;
  count: number;
  accent?: string;
  footer?: string;
}

export function StatCard({
  title,
  count,
  accent = "gray",
  footer,
}: StatCardProps) {
  return (
    <Card withBorder shadow="sm" padding="lg" radius="md">
      <Group justify="space-between" align="flex-start">
        <div>
          <Text size="xs" c="dimmed" tt="uppercase" fw={700}>
            {title}
          </Text>
          <Text size="2rem" fw={700} c={accent} mt="xs">
            {count}
          </Text>
        </div>
      </Group>
      {footer && (
        <Text size="xs" c="dimmed" mt="sm">
          {footer}
        </Text>
      )}
    </Card>
  );
}

interface DashboardStatsProps {
  stats: {
    client_count: number;
    finding_count: number;
    engagement_count: number;
    findings_ready: boolean;
    engagements_ready: boolean;
  };
}

export function DashboardStats({ stats }: DashboardStatsProps) {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, md: 3 }} spacing="md" mb="xl">
      <StatCard
        title="Total Clients"
        count={stats.client_count}
        accent="blue"
      />
      <StatCard
        title="Total Findings"
        count={stats.finding_count}
        footer={stats.findings_ready ? undefined : "Coming soon"}
      />
      <StatCard
        title="Engagements In Progress"
        count={stats.engagement_count}
        footer={stats.engagements_ready ? undefined : "Coming soon"}
      />
    </SimpleGrid>
  );
}
