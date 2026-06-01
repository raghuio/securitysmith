import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Group,
  Modal,
  PasswordInput,
  Stack,
  Text,
  TextInput,
} from "@mantine/core";
import {
  decryptImportToTemp,
  executeImport,
  isImportEncrypted,
  previewImport,
} from "../api/portability";
import type { ConflictResolution, ImportPreview } from "../api/portability";

interface Props {
  opened: boolean;
  onClose: () => void;
}

export function ImportWizard({ opened, onClose }: Props) {
  const [importPath, setImportPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(
    null,
  );
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [isEncrypted, setIsEncrypted] = useState(false);
  const [password, setPassword] = useState("");
  const [effectivePath, setEffectivePath] = useState("");

  // Re-check encryption status whenever the selected path changes.
  useEffect(() => {
    let cancelled = false;
    setIsEncrypted(false);
    setEffectivePath("");
    setImportPreview(null);
    if (!importPath.trim()) {
      return;
    }
    (async () => {
      try {
        const encrypted = await isImportEncrypted(importPath.trim());
        if (cancelled) return;
        setIsEncrypted(encrypted);
      } catch {
        if (!cancelled) setIsEncrypted(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [importPath]);

  const handlePreview = async () => {
    if (!importPath.trim()) return;
    try {
      let path = importPath.trim();
      if (isEncrypted) {
        if (!password) {
          setImportStatus(
            "This export is encrypted. Please enter the export password.",
          );
          return;
        }
        path = await decryptImportToTemp(path, password);
        setEffectivePath(path);
      } else {
        setEffectivePath(path);
      }
      const preview = await previewImport(path);
      setImportPreview(preview);
    } catch (e) {
      setImportStatus(`Preview failed: ${String(e)}`);
    }
  };

  const handleImport = async () => {
    if (!importPath.trim()) return;
    if (isEncrypted && !password) {
      setImportStatus(
        "This export is encrypted. Please enter the export password.",
      );
      return;
    }
    setImporting(true);
    try {
      let path = effectivePath;
      if (!path) {
        path = isEncrypted
          ? await decryptImportToTemp(importPath.trim(), password)
          : importPath.trim();
        setEffectivePath(path);
      }
      const resolutions: ConflictResolution[] =
        importPreview?.conflicts.map((c) => ({
          reference_key: c.import_name,
          action: "rename",
        })) || [];
      const result = await executeImport(path, resolutions);
      setImportStatus(
        `Imported ${Object.values(result.imported).reduce((a, b) => a + b, 0)} entities.`,
      );
    } catch (e) {
      setImportStatus(`Import failed: ${String(e)}`);
    } finally {
      setImporting(false);
    }
  };

  const handleClose = () => {
    setImportPath("");
    setImportPreview(null);
    setImportStatus(null);
    setIsEncrypted(false);
    setPassword("");
    setEffectivePath("");
    onClose();
  };

  return (
    <Modal opened={opened} onClose={handleClose} title="Import Vault" size="md">
      <Stack>
        <Text c="dimmed" size="sm">
          Import data from a previously exported JSON file.
        </Text>
        <TextInput
          label="Import file path"
          placeholder="/home/username/vault-export.ssexport"
          value={importPath}
          onChange={(e) => setImportPath(e.currentTarget.value)}
        />
        {isEncrypted && (
          <PasswordInput
            label="Export password"
            placeholder="Password used to encrypt this export"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
          />
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={handleClose}>
            Cancel
          </Button>
          <Button
            onClick={handlePreview}
            disabled={
              !importPath.trim() || (isEncrypted && password.length < 1)
            }
          >
            Preview
          </Button>
        </Group>
        {importPreview && importPreview.conflicts.length > 0 && (
          <Alert color="yellow" variant="light">
            {importPreview.conflicts.length} conflict(s) detected:{" "}
            {importPreview.conflicts.map((c) => c.import_name).join(", ")}
          </Alert>
        )}
        {importStatus && (
          <Alert color="blue" variant="light">
            {importStatus}
          </Alert>
        )}
        {importPreview && (
          <Button
            onClick={handleImport}
            loading={importing}
            disabled={
              !importPath.trim() ||
              !importPreview.compatible ||
              (isEncrypted && password.length < 1)
            }
          >
            Execute Import
          </Button>
        )}
      </Stack>
    </Modal>
  );
}
