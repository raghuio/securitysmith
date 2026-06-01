import { useState, useEffect } from "react";
import {
  Alert,
  Button,
  Checkbox,
  Collapse,
  Group,
  Modal,
  PasswordInput,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import {
  getExportTree,
  createExport,
  createEncryptedExport,
  type ExportTree,
  type ExportTreeClient,
  type ExportTreeEngagement,
  type ExportTreeTemplate,
} from "../api/portability";

interface Props {
  opened: boolean;
  onClose: () => void;
}

export function ExportWizard({ opened, onClose }: Props) {
  const [tree, setTree] = useState<ExportTree | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [savePath, setSavePath] = useState("");
  const [encrypt, setEncrypt] = useState(false);
  const [password, setPassword] = useState("");
  const [includeCredentialValues, setIncludeCredentialValues] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const generateRandomPassword = () => {
    // Generate a 20-character password from a 64-symbol alphabet.
    const alphabet =
      "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789!@#$%^&*";
    const buf = new Uint8Array(20);
    crypto.getRandomValues(buf);
    let out = "";
    for (let i = 0; i < buf.length; i++) {
      out += alphabet[buf[i] % alphabet.length];
    }
    setPassword(out);
  };
  const [expandedClients, setExpandedClients] = useState<Set<number>>(
    new Set(),
  );

  const [selectedClients, setSelectedClients] = useState<Set<number>>(
    new Set(),
  );
  const [selectedEngagements, setSelectedEngagements] = useState<Set<number>>(
    new Set(),
  );
  const [selectedFindings, setSelectedFindings] = useState<Set<number>>(
    new Set(),
  );
  const [selectedCredentials, setSelectedCredentials] = useState<Set<number>>(
    new Set(),
  );
  const [selectedDocuments, setSelectedDocuments] = useState<Set<number>>(
    new Set(),
  );
  const [selectedInvoices, setSelectedInvoices] = useState<Set<number>>(
    new Set(),
  );
  const [selectedTemplates, setSelectedTemplates] = useState<Set<number>>(
    new Set(),
  );

  useEffect(() => {
    if (opened) {
      setLoading(true);
      getExportTree()
        .then((data) => {
          setTree(data);
          setExpandedClients(new Set(data.clients.map((c) => c.id)));
        })
        .catch((e) => setStatus(`Failed to load export tree: ${String(e)}`))
        .finally(() => setLoading(false));
    }
  }, [opened]);

  const toggleSet = <T,>(set: Set<T>, value: T): Set<T> => {
    const next = new Set(set);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    return next;
  };

  const toggleClient = (client: ExportTreeClient) => {
    const nextClients = toggleSet(selectedClients, client.id);
    const nextEngagements = new Set(selectedEngagements);
    const nextFindings = new Set(selectedFindings);
    const nextCredentials = new Set(selectedCredentials);
    const nextDocuments = new Set(selectedDocuments);
    const nextInvoices = new Set(selectedInvoices);

    const isSelected = !selectedClients.has(client.id);
    if (isSelected) {
      for (const e of client.engagements) {
        nextEngagements.add(e.id);
        e.finding_ids.forEach((id) => nextFindings.add(id));
        e.credential_ids.forEach((id) => nextCredentials.add(id));
        e.document_ids.forEach((id) => nextDocuments.add(id));
      }
      client.documents.forEach((d) => nextDocuments.add(d.id));
      client.invoices.forEach((i) => nextInvoices.add(i.id));
    } else {
      for (const e of client.engagements) {
        nextEngagements.delete(e.id);
        e.finding_ids.forEach((id) => nextFindings.delete(id));
        e.credential_ids.forEach((id) => nextCredentials.delete(id));
        e.document_ids.forEach((id) => nextDocuments.delete(id));
      }
      client.documents.forEach((d) => nextDocuments.delete(d.id));
      client.invoices.forEach((i) => nextInvoices.delete(i.id));
    }

    setSelectedClients(nextClients);
    setSelectedEngagements(nextEngagements);
    setSelectedFindings(nextFindings);
    setSelectedCredentials(nextCredentials);
    setSelectedDocuments(nextDocuments);
    setSelectedInvoices(nextInvoices);
  };

  const toggleEngagement = (
    client: ExportTreeClient,
    engagement: ExportTreeEngagement,
  ) => {
    const nextEngagements = toggleSet(selectedEngagements, engagement.id);
    const nextFindings = new Set(selectedFindings);
    const nextCredentials = new Set(selectedCredentials);
    const nextDocuments = new Set(selectedDocuments);
    const nextInvoices = new Set(selectedInvoices);

    const isSelected = !selectedEngagements.has(engagement.id);
    if (isSelected) {
      engagement.finding_ids.forEach((id) => nextFindings.add(id));
      engagement.credential_ids.forEach((id) => nextCredentials.add(id));
      engagement.document_ids.forEach((id) => nextDocuments.add(id));
    } else {
      engagement.finding_ids.forEach((id) => nextFindings.delete(id));
      engagement.credential_ids.forEach((id) => nextCredentials.delete(id));
      engagement.document_ids.forEach((id) => nextDocuments.delete(id));
    }

    setSelectedEngagements(nextEngagements);
    setSelectedFindings(nextFindings);
    setSelectedCredentials(nextCredentials);
    setSelectedDocuments(nextDocuments);

    // Update client selection based on engagement state
    const allEngagementsSelected = client.engagements.every((e) =>
      nextEngagements.has(e.id),
    );
    const allDocsSelected = client.documents.every((d) =>
      nextDocuments.has(d.id),
    );
    const allInvoicesSelected = client.invoices.every((i) =>
      nextInvoices.has(i.id),
    );
    const nextClients = new Set(selectedClients);
    if (allEngagementsSelected && allDocsSelected && allInvoicesSelected) {
      nextClients.add(client.id);
    } else {
      nextClients.delete(client.id);
    }
    setSelectedClients(nextClients);
  };

  const toggleTemplate = (template: ExportTreeTemplate) => {
    setSelectedTemplates((prev) => toggleSet(prev, template.id));
  };

  const handleExport = async () => {
    if (!savePath.trim()) {
      setStatus("Please enter a save path.");
      return;
    }
    if (encrypt && password.length < 8) {
      setStatus("Encryption password must be at least 8 characters.");
      return;
    }
    setExporting(true);
    setStatus(null);
    try {
      const selection = {
        client_ids: Array.from(selectedClients),
        engagement_ids: Array.from(selectedEngagements),
        finding_ids: Array.from(selectedFindings),
        credential_ids: Array.from(selectedCredentials),
        document_ids: Array.from(selectedDocuments),
        invoice_ids: Array.from(selectedInvoices),
        template_ids: Array.from(selectedTemplates),
      };

      const result = encrypt
        ? await createEncryptedExport(
            selection,
            includeCredentialValues,
            savePath.trim(),
            password,
          )
        : await createExport(
            selection,
            includeCredentialValues,
            savePath.trim(),
          );

      const counts = Object.entries(result.entity_counts)
        .map(([k, v]) => `${v} ${k}`)
        .join(", ");
      setStatus(`Exported to ${result.file_path} (${counts})`);
    } catch (e) {
      setStatus(`Export failed: ${String(e)}`);
    } finally {
      setExporting(false);
    }
  };

  const handleClose = () => {
    setStatus(null);
    setSavePath("");
    setPassword("");
    setEncrypt(false);
    setIncludeCredentialValues(false);
    setSelectedClients(new Set());
    setSelectedEngagements(new Set());
    setSelectedFindings(new Set());
    setSelectedCredentials(new Set());
    setSelectedDocuments(new Set());
    setSelectedInvoices(new Set());
    setSelectedTemplates(new Set());
    onClose();
  };

  const totalSelected =
    selectedClients.size +
    selectedEngagements.size +
    selectedFindings.size +
    selectedCredentials.size +
    selectedDocuments.size +
    selectedInvoices.size +
    selectedTemplates.size;

  return (
    <Modal opened={opened} onClose={handleClose} title="Export Data" size="lg">
      <Stack gap="md">
        {loading && <Text c="dimmed">Loading export tree…</Text>}

        {tree && (
          <>
            <Stack gap="xs">
              <Title order={5}>Clients</Title>
              {tree.clients.map((client) => (
                <Stack key={client.id} gap={2}>
                  <Group gap="xs">
                    <Checkbox
                      checked={selectedClients.has(client.id)}
                      onChange={() => toggleClient(client)}
                      label={
                        <Text fw={600}>
                          {client.name} ({client.engagements.length}{" "}
                          engagements)
                        </Text>
                      }
                    />
                    <Button
                      size="xs"
                      variant="subtle"
                      onClick={() =>
                        setExpandedClients((prev) => toggleSet(prev, client.id))
                      }
                    >
                      {expandedClients.has(client.id) ? "Collapse" : "Expand"}
                    </Button>
                  </Group>

                  <Collapse expanded={expandedClients.has(client.id)}>
                    <Stack gap={2} pl="md">
                      {client.engagements.map((eng) => (
                        <Stack key={eng.id} gap={2}>
                          <Checkbox
                            checked={selectedEngagements.has(eng.id)}
                            onChange={() => toggleEngagement(client, eng)}
                            label={
                              <Text size="sm">
                                {eng.name} ({eng.finding_count} findings,{" "}
                                {eng.credential_count} credentials,{" "}
                                {eng.document_count} docs)
                              </Text>
                            }
                          />
                        </Stack>
                      ))}
                      {client.documents.map((doc) => (
                        <Checkbox
                          key={doc.id}
                          checked={selectedDocuments.has(doc.id)}
                          onChange={() =>
                            setSelectedDocuments((prev) =>
                              toggleSet(prev, doc.id),
                            )
                          }
                          label={
                            <Text size="sm">
                              {doc.name} ({doc.document_type})
                            </Text>
                          }
                        />
                      ))}
                      {client.invoices.map((inv) => (
                        <Checkbox
                          key={inv.id}
                          checked={selectedInvoices.has(inv.id)}
                          onChange={() =>
                            setSelectedInvoices((prev) =>
                              toggleSet(prev, inv.id),
                            )
                          }
                          label={<Text size="sm">{inv.invoice_number}</Text>}
                        />
                      ))}
                    </Stack>
                  </Collapse>
                </Stack>
              ))}

              {tree.templates.length > 0 && (
                <>
                  <Title order={5} mt="sm">
                    Custom Templates
                  </Title>
                  {tree.templates.map((template) => (
                    <Checkbox
                      key={template.id}
                      checked={selectedTemplates.has(template.id)}
                      onChange={() => toggleTemplate(template)}
                      label={
                        <Text size="sm">
                          {template.name} ({template.category})
                        </Text>
                      }
                    />
                  ))}
                </>
              )}
            </Stack>

            <Group gap="sm">
              <Switch
                label="Include credential values"
                checked={includeCredentialValues}
                onChange={(e) =>
                  setIncludeCredentialValues(e.currentTarget.checked)
                }
              />
              <Switch
                label="Encrypt export"
                checked={encrypt}
                onChange={(e) => setEncrypt(e.currentTarget.checked)}
              />
            </Group>

            {encrypt && (
              <Group align="flex-end" gap="xs" wrap="nowrap">
                <PasswordInput
                  style={{ flex: 1 }}
                  label="Export password"
                  placeholder="At least 8 characters"
                  value={password}
                  onChange={(e) => setPassword(e.currentTarget.value)}
                />
                <Button variant="default" onClick={generateRandomPassword}>
                  Generate
                </Button>
              </Group>
            )}

            <TextInput
              label={`Save path (${totalSelected} items selected)`}
              placeholder="/home/user/export.ssexport"
              value={savePath}
              onChange={(e) => setSavePath(e.currentTarget.value)}
            />

            <Group justify="flex-end">
              <Button variant="default" onClick={handleClose}>
                Cancel
              </Button>
              <Button
                onClick={handleExport}
                loading={exporting}
                disabled={totalSelected === 0 || !savePath.trim()}
              >
                Export
              </Button>
            </Group>
          </>
        )}

        {status && (
          <Alert
            color={status.startsWith("Exported") ? "green" : "red"}
            variant="light"
          >
            {status}
          </Alert>
        )}
      </Stack>
    </Modal>
  );
}
