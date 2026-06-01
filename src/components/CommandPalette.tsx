import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Modal,
  Stack,
  Text,
  TextInput,
  UnstyledButton,
} from "@mantine/core";
import { IconPlus, IconSearch, IconBriefcase } from "@tabler/icons-react";
import { globalSearch, type SearchResult } from "../api/search";
import type { Client } from "../api/clients";
import type { Engagement } from "../api/engagements";
import type { Finding } from "../api/findings";

interface CommandPaletteProps {
  opened: boolean;
  onClose: () => void;
  onCreateClient: () => void;
  onSelectClient: (client: Client) => void;
  onCreateEngagement: (clientId?: number) => void;
  onSelectEngagement: (engagement: Engagement) => void;
  onCreateFinding: (engagementId?: number) => void;
  onSelectFinding: (finding: Finding) => void;
}

export function CommandPalette({
  opened,
  onClose,
  onCreateClient,
  onSelectClient,
  onCreateEngagement,
  onSelectEngagement,
  onCreateFinding,
  onSelectFinding,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!opened) {
      return;
    }

    const timeoutId = window.setTimeout(async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await globalSearch(query.trim() || "*", 20);
        setResults(data);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }, 120);

    return () => window.clearTimeout(timeoutId);
  }, [opened, query]);

  const handleClose = () => {
    setQuery("");
    setError(null);
    onClose();
  };

  const handleCreateClient = () => {
    handleClose();
    onCreateClient();
  };

  const handleCreateEngagement = () => {
    handleClose();
    onCreateEngagement();
  };

  const handleCreateFinding = () => {
    handleClose();
    onCreateFinding();
  };

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title="Command palette"
      centered
      size="lg"
    >
      <Stack gap="sm">
        <TextInput
          leftSection={<IconSearch size={16} />}
          placeholder="Search clients, engagements, and findings"
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
          autoFocus
        />

        <Stack gap={4}>
          <Button
            variant="light"
            justify="flex-start"
            leftSection={<IconPlus size={16} />}
            onClick={handleCreateClient}
          >
            New client
          </Button>
          <Button
            variant="light"
            justify="flex-start"
            leftSection={<IconBriefcase size={16} />}
            onClick={handleCreateEngagement}
          >
            New engagement
          </Button>
          <Button
            variant="light"
            justify="flex-start"
            leftSection={<IconPlus size={16} />}
            onClick={handleCreateFinding}
          >
            New finding
          </Button>
        </Stack>

        {error && (
          <Text size="sm" c="red">
            {error}
          </Text>
        )}

        {results.length > 0 && (
          <Stack gap={4}>
            {results.map((result) => (
              <UnstyledButton
                key={`${result.entity_type}-${result.entity_id}`}
                p="xs"
                style={{
                  borderRadius: 6,
                  border: "1px solid var(--mantine-color-default-border)",
                }}
                onClick={() => {
                  handleClose();
                  if (result.entity_type === "client") {
                    onSelectClient({
                      id: result.entity_id,
                      name: result.title,
                    } as Client);
                  } else if (result.entity_type === "engagement") {
                    onSelectEngagement({
                      id: result.entity_id,
                      name: result.title,
                    } as Engagement);
                  } else if (result.entity_type === "finding") {
                    onSelectFinding({
                      id: result.entity_id,
                      title: result.title,
                    } as Finding);
                  }
                }}
              >
                <Group justify="space-between" wrap="nowrap">
                  <div>
                    <Text fw={600}>{result.title}</Text>
                    <Text size="sm" c="dimmed">
                      {result.entity_type} · {result.subtitle}
                    </Text>
                  </div>
                </Group>
              </UnstyledButton>
            ))}
          </Stack>
        )}

        {!loading && results.length === 0 && !error && query.trim() && (
          <Text size="sm" c="dimmed" ta="center" py="sm">
            No results found.
          </Text>
        )}
      </Stack>
    </Modal>
  );
}
