import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Stack,
  Text,
  Title,
  ActionIcon,
  Tooltip,
} from "@mantine/core";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { listReports, archiveReport, type Report } from "../api/reports";
import { ReportBuilder } from "./ReportBuilder";

export function ReportList() {
  const [reports, setReports] = useState<Report[]>([]);
  const [builderOpen, setBuilderOpen] = useState(false);
  const [editing, setEditing] = useState<Report | null>(null);
  const [engagementId, setEngagementId] = useState(1);

  const load = async () => {
    try {
      const data = await listReports();
      setReports(data);
    } catch (e) {
      console.error(e);
    }
  };
  useEffect(() => {
    load();
  }, []);

  const handleEdit = (r: Report) => {
    setEditing(r);
    setEngagementId(r.engagement_id);
    setBuilderOpen(true);
  };

  const handleCreate = () => {
    setEditing(null);
    setBuilderOpen(true);
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Reports</Title>
        <Group>
          <Button onClick={load}>Refresh</Button>
          <Button leftSection={<IconPlus size={16} />} onClick={handleCreate}>
            New Report
          </Button>
        </Group>
      </Group>
      {reports.map((r) => (
        <Group key={r.id} justify="space-between">
          <Text>
            {r.name} · {r.client_name} · {r.status}
          </Text>
          <Group gap="xs">
            <Tooltip label="Edit">
              <ActionIcon variant="light" onClick={() => handleEdit(r)}>
                <IconEdit size={16} />
              </ActionIcon>
            </Tooltip>
            <Button
              variant="default"
              size="xs"
              onClick={() => archiveReport(r.id).then(load)}
            >
              Archive
            </Button>
          </Group>
        </Group>
      ))}
      {reports.length === 0 && <Text c="dimmed">No reports yet.</Text>}

      <ReportBuilder
        opened={builderOpen}
        onClose={() => setBuilderOpen(false)}
        engagementId={engagementId}
        report={editing}
        onSaved={load}
      />
    </Stack>
  );
}
