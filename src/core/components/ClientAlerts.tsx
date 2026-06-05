import { useEffect, useState } from "react";
import { Badge, Button, Group, Paper, Stack, Text, Title } from "@mantine/core";
import { getClientAlerts, type ClientAlert } from "../../extensions/news/api";

export function ClientAlerts() {
  const [alerts, setAlerts] = useState<ClientAlert[]>([]);
  const [loading, setLoading] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const data = await getClientAlerts();
      setAlerts(data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Client Alerts</Title>
        <Button onClick={load} loading={loading}>
          Refresh
        </Button>
      </Group>
      {alerts.length === 0 && (
        <Text c="dimmed">No client alerts at this time.</Text>
      )}
      {alerts.map((alert) => (
        <Paper key={`${alert.article_id}-${alert.client_id}`} withBorder p="sm">
          <Group gap="xs">
            <Badge size="sm" color="red">
              Match
            </Badge>
            <Text fw={600} size="sm">
              {alert.client_name}
            </Text>
          </Group>
          <Text size="sm" mt="xs">
            <a
              href={alert.article_link || undefined}
              target="_blank"
              rel="noreferrer"
            >
              {alert.article_title}
            </a>
          </Text>
          <Group gap="xs" mt="xs">
            {alert.matched_tags.map((tag) => (
              <Badge key={tag} size="xs" variant="outline">
                {tag}
              </Badge>
            ))}
          </Group>
        </Paper>
      ))}
    </Stack>
  );
}
