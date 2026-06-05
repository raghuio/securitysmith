import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Drawer,
  Group,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Paper,
  Checkbox,
  ActionIcon,
} from "@mantine/core";
import { IconArrowUp, IconArrowDown } from "@tabler/icons-react";
import { createReport, updateReport, generateReportPdf } from "../api";
import type { Report } from "../api";
import { listFindings } from "../../../core/api/findings";
import type { Finding } from "../../../core/api/findings";

import { listEngagements } from "../../../core/api/engagements";
import type { Engagement } from "../../../core/api/engagements";

interface Props {
  opened: boolean;
  onClose: () => void;
  engagementId?: number;
  report?: Report | null;
  onSaved: () => void;
}

export function ReportBuilder({
  opened,
  onClose,
  engagementId: initialEngagementId,
  report,
  onSaved,
}: Props) {
  const isEdit = !!report;
  const [name, setName] = useState(report?.name || "");
  const [executiveSummary, setExecutiveSummary] = useState(
    report?.executive_summary || "",
  );
  const [appendix, setAppendix] = useState(report?.appendix || "");
  const [findingIds, setFindingIds] = useState<string[]>(
    report?.included_finding_ids.map((id) => String(id)) || [],
  );
  const [findings, setFindings] = useState<Finding[]>([]);
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const [selectedEngagementId, setSelectedEngagementId] = useState<
    number | undefined
  >(initialEngagementId);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (opened) {
      setName(report?.name || "");
      setExecutiveSummary(report?.executive_summary || "");
      setAppendix(report?.appendix || "");
      setFindingIds(report?.included_finding_ids.map(String) || []);
      setSelectedEngagementId(initialEngagementId);
      listEngagements().then(setEngagements).catch(console.error);
    }
  }, [opened, report, initialEngagementId]);

  useEffect(() => {
    if (selectedEngagementId) {
      listFindings({ engagementId: selectedEngagementId })
        .then((page) => setFindings(page.items))
        .catch(console.error);
    }
  }, [selectedEngagementId]);

  const handleSave = async () => {
    if (!name.trim() || (!isEdit && !selectedEngagementId)) {
      setError("Report name and engagement are required.");
      return;
    }
    setError(null);
    setLoading(true);
    try {
      const ids = findingIds.map(Number);
      const eid = isEdit ? report!.engagement_id : selectedEngagementId || 1;
      if (isEdit && report) {
        await updateReport(report.id, {
          name: name.trim(),
          executive_summary: executiveSummary,
          appendix,
          included_finding_ids: ids,
        });
      } else {
        await createReport(eid, name.trim(), executiveSummary, appendix, ids);
      }
      onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleGeneratePdf = async () => {
    if (!report) return;
    setLoading(true);
    try {
      const path = await generateReportPdf(report.id);
      setError(null);
      alert(`PDF generated: ${path}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={isEdit ? "Edit Report" : "New Report"}
      position="right"
      size="xl"
    >
      <Stack gap="md">
        <Select
          label="Engagement"
          value={selectedEngagementId ? String(selectedEngagementId) : ""}
          onChange={(v) => setSelectedEngagementId(v ? Number(v) : undefined)}
          data={engagements.map((e) => ({
            value: String(e.id),
            label: `${e.name} · ${e.client_name}`,
          }))}
          disabled={isEdit}
          searchable
        />
        <TextInput
          label="Report Name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
        />
        <Textarea
          label="Executive Summary"
          value={executiveSummary}
          onChange={(e) => setExecutiveSummary(e.currentTarget.value)}
          autosize
          minRows={5}
          maxRows={20}
        />
        <Textarea
          label="Appendix"
          value={appendix}
          onChange={(e) => setAppendix(e.currentTarget.value)}
          autosize
          minRows={3}
          maxRows={15}
        />
        <Text fw={500} size="sm">
          Included Findings
        </Text>
        <Stack gap="xs">
          {findings.map((f) => {
            const selected = findingIds.includes(String(f.id));
            const index = findingIds.indexOf(String(f.id));
            return (
              <Paper key={f.id} withBorder p="xs" radius="sm">
                <Group justify="space-between">
                  <Group gap="xs">
                    <Checkbox
                      checked={selected}
                      onChange={(event) => {
                        const checked = event.currentTarget.checked;
                        setFindingIds((prev) => {
                          if (checked) return [...prev, String(f.id)];
                          return prev.filter((id) => id !== String(f.id));
                        });
                      }}
                    />
                    <Text size="sm">
                      {f.title} ({f.severity})
                    </Text>
                  </Group>
                  {selected && (
                    <Group gap={2}>
                      <ActionIcon
                        size="sm"
                        variant="subtle"
                        disabled={index <= 0}
                        onClick={() => {
                          setFindingIds((prev) => {
                            const next = [...prev];
                            [next[index], next[index - 1]] = [
                              next[index - 1],
                              next[index],
                            ];
                            return next;
                          });
                        }}
                      >
                        <IconArrowUp size={14} />
                      </ActionIcon>
                      <ActionIcon
                        size="sm"
                        variant="subtle"
                        disabled={index >= findingIds.length - 1}
                        onClick={() => {
                          setFindingIds((prev) => {
                            const next = [...prev];
                            [next[index], next[index + 1]] = [
                              next[index + 1],
                              next[index],
                            ];
                            return next;
                          });
                        }}
                      >
                        <IconArrowDown size={14} />
                      </ActionIcon>
                    </Group>
                  )}
                </Group>
              </Paper>
            );
          })}
        </Stack>
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          {isEdit && (
            <Button
              variant="light"
              onClick={handleGeneratePdf}
              loading={loading}
            >
              Generate PDF
            </Button>
          )}
          <Button onClick={handleSave} loading={loading}>
            Save
          </Button>
        </Group>
      </Stack>
    </Drawer>
  );
}
