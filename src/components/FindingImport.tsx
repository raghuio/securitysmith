import { useState } from "react";
import {
  Button,
  Group,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
  Alert,
  Badge,
  Checkbox,
  Paper,
} from "@mantine/core";
import { parseImportFile, commitImport } from "../api/findings_import";
import type {
  ImportFormat,
  ImportPreview,
  CsvColumnMapping,
} from "../api/findings_import";

interface Props {
  engagementId: number;
  opened: boolean;
  onClose: () => void;
  onImported: () => void;
}

export function FindingImport({
  engagementId,
  opened,
  onClose,
  onImported,
}: Props) {
  const [filePath, setFilePath] = useState("");
  const [format, setFormat] = useState<ImportFormat>("nessus");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(
    new Set(),
  );

  const [csvTitle, setCsvTitle] = useState(0);
  const [csvSeverity, setCsvSeverity] = useState(1);

  const handlePreview = async () => {
    if (!filePath.trim()) {
      setError("Please select a file.");
      return;
    }
    setError(null);
    setLoading(true);
    try {
      const mapping: CsvColumnMapping | undefined =
        format === "csv"
          ? { title: csvTitle, severity: csvSeverity }
          : undefined;
      const result = await parseImportFile(
        filePath.trim(),
        format,
        engagementId,
        mapping,
      );
      setPreview(result);
      setSelectedIndices(new Set(result.findings.map((_, i) => i)));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleCommit = async () => {
    if (!preview) return;
    const chosen = preview.findings.filter((_, i) => selectedIndices.has(i));
    if (chosen.length === 0) return;
    setLoading(true);
    try {
      await commitImport(engagementId, chosen);
      onImported();
      handleClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    setFilePath("");
    setPreview(null);
    setError(null);
    setSelectedIndices(new Set());
    onClose();
  };

  const toggleIndex = (i: number) => {
    const next = new Set(selectedIndices);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    setSelectedIndices(next);
  };

  return (
    <Modal
      opened={opened}
      onClose={handleClose}
      title="Import Findings"
      size="xl"
    >
      <Stack>
        {!preview && (
          <>
            <TextInput
              label="File path"
              placeholder="/path/to/file"
              value={filePath}
              onChange={(e) => setFilePath(e.currentTarget.value)}
            />
            <Select
              label="Format"
              value={format}
              onChange={(v) => setFormat((v as ImportFormat) || "nessus")}
              data={[
                { value: "nessus", label: "Nessus" },
                { value: "burp", label: "Burp Suite" },
                { value: "zap_json", label: "OWASP ZAP (JSON)" },
                { value: "nmap", label: "Nmap" },
                { value: "nuclei", label: "Nuclei" },
                { value: "csv", label: "CSV" },
              ]}
            />
            {format === "csv" && (
              <Group>
                <TextInput
                  label="Title Column"
                  value={String(csvTitle)}
                  onChange={(e) =>
                    setCsvTitle(Number(e.currentTarget.value) || 0)
                  }
                  style={{ width: 120 }}
                />
                <TextInput
                  label="Severity Column"
                  value={String(csvSeverity)}
                  onChange={(e) =>
                    setCsvSeverity(Number(e.currentTarget.value) || 1)
                  }
                  style={{ width: 120 }}
                />
              </Group>
            )}
            {error && (
              <Alert color="red" variant="light">
                {error}
              </Alert>
            )}
            <Group justify="flex-end">
              <Button variant="default" onClick={handleClose}>
                Cancel
              </Button>
              <Button onClick={handlePreview} loading={loading}>
                Preview
              </Button>
            </Group>
          </>
        )}

        {preview && (
          <>
            <Text size="sm">
              Parsed {preview.total_parsed} findings. Duplicates detected:{" "}
              {preview.duplicates_found}.
            </Text>
            <Text size="sm" c="dimmed">
              Format: {preview.format}
            </Text>
            <Stack style={{ maxHeight: 400, overflowY: "auto" }}>
              {preview.findings.map((f, i) => (
                <Paper key={i} withBorder p="sm">
                  <Group>
                    <Checkbox
                      checked={selectedIndices.has(i)}
                      onChange={() => toggleIndex(i)}
                      label={
                        <Text fw={600} size="sm">
                          {f.title}
                        </Text>
                      }
                    />
                  </Group>
                  <Group gap="xs" mt="xs">
                    {f.severity && (
                      <Badge size="xs" color="red">
                        {f.severity}
                      </Badge>
                    )}
                    {f.source_tool && (
                      <Badge size="xs" variant="outline">
                        {f.source_tool}
                      </Badge>
                    )}
                    {f.is_duplicate && (
                      <Badge size="xs" color="yellow">
                        Duplicate
                      </Badge>
                    )}
                  </Group>
                  <Text size="xs" c="dimmed" mt="xs">
                    {f.overview}
                  </Text>
                </Paper>
              ))}
            </Stack>
            {error && (
              <Alert color="red" variant="light">
                {error}
              </Alert>
            )}
            <Group justify="flex-end">
              <Button variant="default" onClick={() => setPreview(null)}>
                Back
              </Button>
              <Button onClick={handleCommit} loading={loading}>
                Import {selectedIndices.size} selected
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Modal>
  );
}
