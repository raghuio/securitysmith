import { useEffect, useState } from "react";
import { Button, Group, Stack, Text, Title, Alert } from "@mantine/core";
import { listNewsArticles, refreshFeeds, type NewsArticle } from "../api";
import { ClientAlerts } from "../../../core/components/ClientAlerts";

export function NewsFeed() {
  const [articles, setArticles] = useState<NewsArticle[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState<string | null>(null);

  const load = async () => {
    try {
      const data = await listNewsArticles();
      setArticles(data);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    setRefreshStatus(null);
    try {
      const result = await refreshFeeds();
      setRefreshStatus(
        `Fetched ${result.new_articles} new articles.` +
          (result.errors.length > 0
            ? ` Errors: ${result.errors.join("; ")}`
            : ""),
      );
      await load();
    } catch (e) {
      setRefreshStatus(String(e));
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>News</Title>
        <Group gap="xs">
          <Button onClick={handleRefresh} loading={refreshing}>
            Refresh Feeds
          </Button>
          <Button variant="default" onClick={load}>
            Reload
          </Button>
        </Group>
      </Group>
      {refreshStatus && (
        <Alert color="blue" variant="light">
          {refreshStatus}
        </Alert>
      )}
      {articles.map((a) => (
        <Stack key={a.id} gap={2}>
          <Text fw={600}>{a.title}</Text>
          <Text size="xs" c="dimmed">
            {a.feed_name} ·{" "}
            {a.matched_clients.length > 0
              ? `Matches ${a.matched_clients.length} clients`
              : "No matches"}
          </Text>
        </Stack>
      ))}
      {articles.length === 0 && <Text c="dimmed">No articles yet.</Text>}
      <ClientAlerts />
    </Stack>
  );
}
