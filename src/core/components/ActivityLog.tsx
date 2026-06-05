import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Stack,
  Table,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { listActivityLog, type ActivityLogEntry } from "../api/activity_log";

export function ActivityLog() {
  const [entries, setEntries] = useState<ActivityLogEntry[]>([]);
  const [search, setSearch] = useState("");
  const load = async () => {
    try {
      const data = await listActivityLog({ search: search || undefined });
      setEntries(data);
    } catch (e) {
      console.error(e);
    }
  };
  useEffect(() => {
    load();
  }, []);
  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Activity Log</Title>
        <Button onClick={load}>Refresh</Button>
      </Group>
      <TextInput
        placeholder="Search activity log..."
        value={search}
        onChange={(e) => setSearch(e.currentTarget.value)}
        onKeyDown={(e) => e.key === "Enter" && load()}
      />
      <Table>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Time</Table.Th>
            <Table.Th>Table</Table.Th>
            <Table.Th>Action</Table.Th>
            <Table.Th>Record</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {entries.map((entry) => (
            <Table.Tr key={entry.id}>
              <Table.Td>
                {new Date(entry.timestamp * 1000).toLocaleString()}
              </Table.Td>
              <Table.Td>{entry.table_name}</Table.Td>
              <Table.Td>{entry.action}</Table.Td>
              <Table.Td>{entry.record_id}</Table.Td>
            </Table.Tr>
          ))}
        </Table.Tbody>
      </Table>
      {entries.length === 0 && <Text c="dimmed">No log entries found.</Text>}
    </Stack>
  );
}
