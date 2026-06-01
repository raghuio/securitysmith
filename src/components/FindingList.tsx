import { useState, useEffect } from "react";
import {
  ActionIcon,
  Badge,
  Button,
  Group,
  Modal,
  Paper,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { TagGroup } from "./shared";
import {
  IconArrowLeft,
  IconEdit,
  IconCopy,
  IconSearch,
  IconTrash,
  IconPlus,
  IconEye,
  IconUpload,
} from "@tabler/icons-react";
import {
  listFindings,
  archiveFinding,
  duplicateFinding,
} from "../api/findings";
import type { Finding, Severity, FindingStatus } from "../api/findings";
import { FindingDetail } from "./FindingDetail";
import { FindingImport } from "./FindingImport";

interface Props {
  engagementId?: number;
  clientId?: number;
  onBack: () => void;
  onEdit: (finding: Finding) => void;
  onCreate: (engagementId?: number) => void;
  onArchived: () => void;
  refreshKey: number;
}

const SEVERITY_COLORS: Record<Severity, string> = {
  critical: "red",
  high: "orange",
  medium: "yellow",
  low: "blue",
  informational: "gray",
};

export function FindingList({
  engagementId,
  clientId,
  onBack,
  onEdit,
  onCreate,
  onArchived,
  refreshKey,
}: Props) {
  const [findings, setFindings] = useState<Finding[]>([]);
  const [total, setTotal] = useState(0);
  const [search, setSearch] = useState("");
  const [severityFilter, setSeverityFilter] = useState<Severity | "">("");
  const [statusFilter, setStatusFilter] = useState<FindingStatus | "">("");
  const [loading, setLoading] = useState(false);
  const [archiveTarget, setArchiveTarget] = useState<Finding | null>(null);
  const [detailTarget, setDetailTarget] = useState<Finding | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [offset, setOffset] = useState(0);
  const limit = 20;

  const load = async (newOffset?: number) => {
    setLoading(true);
    try {
      const page = await listFindings({
        engagementId,
        clientId,
        search: search.trim() || undefined,
        severity: (severityFilter || undefined) as Severity | undefined,
        status: (statusFilter || undefined) as FindingStatus | undefined,
        offset: newOffset ?? offset,
        limit,
      });
      setFindings(page.items);
      setTotal(page.total);
      setOffset(page.offset);
    } catch (e) {
      console.error("Failed to load findings:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, severityFilter, statusFilter, refreshKey]);

  const handleArchive = async () => {
    if (!archiveTarget) return;
    try {
      await archiveFinding(archiveTarget.id);
      setArchiveTarget(null);
      onArchived();
      load(offset);
    } catch (e) {
      console.error("Failed to archive finding:", e);
    }
  };

  const handleDuplicate = async (finding: Finding) => {
    try {
      await duplicateFinding(finding.id);
      load(offset);
    } catch (e) {
      console.error("Failed to duplicate finding:", e);
    }
  };

  return (
    <Stack gap="md" p="md">
      <Group justify="space-between" align="center">
        <Group gap="sm">
          <ActionIcon variant="light" onClick={onBack}>
            <IconArrowLeft size={18} />
          </ActionIcon>
          <Title order={4}>
            {engagementId ? "Engagement Findings" : "Findings"}
          </Title>
        </Group>
        <Text size="sm" c="dimmed">
          {total} {total === 1 ? "finding" : "findings"}
        </Text>
      </Group>

      <Group grow align="flex-start">
        <TextInput
          placeholder="Search findings..."
          leftSection={<IconSearch size={16} />}
          value={search}
          onChange={(e) => setSearch(e.currentTarget.value)}
          disabled={loading}
        />
        <Select
          placeholder="Severity"
          data={[
            { value: "", label: "All" },
            { value: "critical", label: "Critical" },
            { value: "high", label: "High" },
            { value: "medium", label: "Medium" },
            { value: "low", label: "Low" },
            { value: "informational", label: "Info" },
          ]}
          value={severityFilter}
          onChange={(val) => setSeverityFilter((val as Severity | "") ?? "")}
          disabled={loading}
          clearable
        />
        <Select
          placeholder="Status"
          data={[
            { value: "", label: "All" },
            { value: "draft", label: "Draft" },
            { value: "confirmed", label: "Confirmed" },
            { value: "reported", label: "Reported" },
            { value: "fixed", label: "Fixed" },
            { value: "accepted", label: "Accepted" },
            { value: "false_positive", label: "False Positive" },
            { value: "wont_fix", label: "Won't Fix" },
          ]}
          value={statusFilter}
          onChange={(val) => setStatusFilter((val as FindingStatus | "") ?? "")}
          disabled={loading}
          clearable
        />
      </Group>

      <Group>
        <Button
          leftSection={<IconPlus size={16} />}
          variant="light"
          onClick={() => onCreate(engagementId)}
          disabled={!engagementId}
        >
          Add finding
        </Button>
        {engagementId && (
          <Button
            leftSection={<IconUpload size={16} />}
            variant="default"
            onClick={() => setImportOpen(true)}
          >
            Import
          </Button>
        )}
      </Group>

      <Stack gap="xs">
        {findings.length === 0 && !loading && (
          <Text c="dimmed" ta="center" py="xl">
            {search.trim() || severityFilter || statusFilter
              ? "No findings match your filters."
              : "No findings yet."}
          </Text>
        )}
        {findings.map((finding) => (
          <Paper key={finding.id} withBorder shadow="xs" p="sm" radius="md">
            <Group justify="space-between" align="flex-start">
              <div style={{ flex: 1 }}>
                <Group gap="sm">
                  <Text fw={600}>{finding.title}</Text>
                  <Badge color={SEVERITY_COLORS[finding.severity]} size="sm">
                    {finding.severity}
                  </Badge>
                  <Badge variant="light" size="sm">
                    {finding.status}
                  </Badge>
                </Group>
                <Text size="sm" c="dimmed">
                  {finding.client_name} · {finding.engagement_name}
                </Text>
                {finding.owasp_category && (
                  <Text size="xs" c="dimmed">
                    {finding.owasp_category}
                  </Text>
                )}
                {finding.tags.length > 0 && (
                  <TagGroup tags={finding.tags} size="xs" />
                )}
              </div>
              <Group gap={4}>
                <ActionIcon
                  variant="light"
                  size="sm"
                  onClick={() => setDetailTarget(finding)}
                  title="View"
                >
                  <IconEye size={14} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  size="sm"
                  onClick={() => handleDuplicate(finding)}
                  title="Duplicate"
                >
                  <IconCopy size={14} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  size="sm"
                  onClick={() => onEdit(finding)}
                  title="Edit"
                >
                  <IconEdit size={14} />
                </ActionIcon>
                <ActionIcon
                  variant="light"
                  size="sm"
                  color="red"
                  onClick={() => setArchiveTarget(finding)}
                  title="Archive"
                >
                  <IconTrash size={14} />
                </ActionIcon>
              </Group>
            </Group>
          </Paper>
        ))}
      </Stack>

      {total > limit && (
        <Group justify="center" gap="sm">
          <Button
            variant="light"
            size="xs"
            disabled={offset === 0}
            onClick={() => load(Math.max(0, offset - limit))}
          >
            Previous
          </Button>
          <Text size="sm" c="dimmed">
            {offset + 1}–{Math.min(offset + limit, total)} of {total}
          </Text>
          <Button
            variant="light"
            size="xs"
            disabled={offset + limit >= total}
            onClick={() => load(offset + limit)}
          >
            Next
          </Button>
        </Group>
      )}

      <Modal
        opened={!!archiveTarget}
        onClose={() => setArchiveTarget(null)}
        title="Archive finding"
        centered
      >
        <Stack>
          <Text>
            Are you sure you want to archive{" "}
            <strong>{archiveTarget?.title}</strong>?
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setArchiveTarget(null)}>
              Cancel
            </Button>
            <Button color="red" onClick={handleArchive}>
              Archive
            </Button>
          </Group>
        </Stack>
      </Modal>

      <FindingDetail
        finding={detailTarget}
        opened={!!detailTarget}
        onClose={() => setDetailTarget(null)}
      />

      {engagementId && (
        <FindingImport
          engagementId={engagementId}
          opened={importOpen}
          onClose={() => setImportOpen(false)}
          onImported={() => load(0)}
        />
      )}
    </Stack>
  );
}
