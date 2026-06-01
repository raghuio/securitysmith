import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Select,
  Stack,
  Table,
  Text,
  Title,
  Badge,
} from "@mantine/core";
import {
  createRetestEngagement,
  listRetestEngagements,
  getRetestComparison,
  bulkUpdateFindingStatus,
  type RetestEngagement,
} from "../api/retests";

export function RetestView({ engagementId }: { engagementId: number }) {
  const [retests, setRetests] = useState<RetestEngagement[]>([]);
  const [comparison, setComparison] = useState<Record<string, unknown>[]>([]);
  const [selectedRetest, setSelectedRetest] = useState<number | null>(null);

  const load = async () => {
    try {
      const data = await listRetestEngagements(engagementId);
      setRetests(data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [engagementId]);

  useEffect(() => {
    if (!selectedRetest) {
      setComparison([]);
      return;
    }
    getRetestComparison(selectedRetest)
      .then(setComparison)
      .catch(console.error);
  }, [selectedRetest]);

  const handleCreateRetest = async () => {
    try {
      await createRetestEngagement(engagementId);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleBulkUpdate = async (findingIds: number[], status: string) => {
    try {
      await bulkUpdateFindingStatus(findingIds, status);
      if (selectedRetest) {
        const data = await getRetestComparison(selectedRetest);
        setComparison(data);
      }
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={5}>Retest & Remediation</Title>
        <Button onClick={handleCreateRetest}>Create Retest</Button>
      </Group>

      {retests.length > 0 && (
        <Select
          placeholder="Select retest engagement"
          data={retests.map((r) => ({
            value: String(r.id),
            label: `${r.name} · ${r.client_name}`,
          }))}
          value={selectedRetest ? String(selectedRetest) : null}
          onChange={(v) => setSelectedRetest(v ? Number(v) : null)}
        />
      )}

      {comparison.length > 0 && (
        <Table highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Finding</Table.Th>
              <Table.Th>Original Severity</Table.Th>
              <Table.Th>Retest Result</Table.Th>
              <Table.Th>Actions</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {comparison.map((row: Record<string, unknown>) => (
              <Table.Tr key={String(row.id)}>
                <Table.Td>{String(row.title)}</Table.Td>
                <Table.Td>
                  <Badge size="sm">{String(row.original_severity)}</Badge>
                </Table.Td>
                <Table.Td>
                  <Badge
                    size="sm"
                    color={
                      row.retest_result === "pass"
                        ? "green"
                        : row.retest_result === "fail"
                          ? "red"
                          : row.retest_result === "partial"
                            ? "yellow"
                            : "gray"
                    }
                  >
                    {String(row.retest_result).replace("_", " ")}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  <Group gap="xs">
                    <Button
                      size="xs"
                      variant="subtle"
                      onClick={() =>
                        handleBulkUpdate([Number(row.id)], "fixed")
                      }
                    >
                      Pass
                    </Button>
                    <Button
                      size="xs"
                      variant="subtle"
                      color="red"
                      onClick={() =>
                        handleBulkUpdate([Number(row.id)], "disputed")
                      }
                    >
                      Fail
                    </Button>
                  </Group>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}

      {retests.length === 0 && (
        <Text c="dimmed">No retest engagements yet.</Text>
      )}
    </Stack>
  );
}
