import { useState, useEffect } from "react";
import {
  Alert,
  Button,
  ColorPicker,
  Group,
  PasswordInput,
  Select,
  Stack,
  Switch,
  Tabs,
  Text,
  TextInput,
} from "@mantine/core";
import { useMantineColorScheme } from "@mantine/core";
import { getSetting, setSetting, testOllamaConnection } from "../api/settings";
import { exportVaultJson } from "../api/portability";
import { previewImport, executeImport } from "../api/portability";
import type { ConflictResolution, ImportPreview } from "../api/portability";
import { rotateRecoveryPhrase, changeMasterPassword } from "../api/auth";
import { testSmtpConnection } from "../../extensions/email/api";
import { useExtensionSettings } from "../extensions/useExtensionSettings";
import { extensions } from "../../extensions";
import type { RecoveryInfo } from "../api/auth";
import { RecoveryPhraseModal } from "./RecoveryPhraseModal";

const PREDEFINED_COLORS = [
  { name: "Blue", hex: "#228be6" },
  { name: "Teal", hex: "#12b886" },
  { name: "Green", hex: "#40c057" },
  { name: "Orange", hex: "#fd7e14" },
  { name: "Red", hex: "#fa5252" },
  { name: "Purple", hex: "#7950f2" },
  { name: "Pink", hex: "#e64980" },
  { name: "Gray", hex: "#868e96" },
];

const DEFAULT_WIDGETS = {
  clients: true,
  engagements: true,
  findings: true,
  credentials: true,
  news: true,
  calendar: true,
  recent_activity: true,
};

const DEFAULT_NAV = {
  dashboard: true,
  clients: true,
  engagements: true,
  templates: true,
  documents: true,
  calendar: true,
  news: true,
  activity_log: true,
  settings: true,
};


// Sub-component for extension toggles
function ExtensionToggleList() {
  const { settings, loaded, toggleExtension } = useExtensionSettings();

  if (!loaded) {
    return <Text size="sm" c="dimmed">Loading extensions...</Text>;
  }

  return (
    <Stack gap="xs">
      {extensions.map((ext) => (
        <Group key={ext.id} justify="space-between" wrap="nowrap">
          <Stack gap={0}>
            <Text size="sm" fw={500}>{ext.name}</Text>
            <Text size="xs" c="dimmed">{ext.description}</Text>
          </Stack>
          <Switch
            checked={settings[ext.id] ?? true}
            onChange={(event) => toggleExtension(ext.id, event.currentTarget.checked)}
            aria-label={`Toggle ${ext.name}`}
          />
        </Group>
      ))}
    </Stack>
  );
}

export function SettingsPage() {
  const { setColorScheme } = useMantineColorScheme();

  const [profileName, setProfileName] = useState("");
  const [brandCompany, setBrandCompany] = useState("");
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [accentColor, setAccentColor] = useState("#228be6");
  const [widgets, setWidgets] = useState(DEFAULT_WIDGETS);
  const [navItems, setNavItems] = useState(DEFAULT_NAV);

  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");

  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);
  const [testingOllama, setTestingOllama] = useState(false);

  const [smtpHost, setSmtpHost] = useState("");
  const [smtpPort, setSmtpPort] = useState("587");
  const [smtpUser, setSmtpUser] = useState("");
  const [smtpPassword, setSmtpPassword] = useState("");
  const [smtpTls, setSmtpTls] = useState(true);
  const [smtpFrom, setSmtpFrom] = useState("");
  const [smtpTestResult, setSmtpTestResult] = useState<string | null>(null);
  const [testingSmtp, setTestingSmtp] = useState(false);

  const [feedbackDays, setFeedbackDays] = useState("7");
  const [retestDays, setRetestDays] = useState("90");

  const [exportPath, setExportPath] = useState("");
  const [exporting, setExporting] = useState(false);
  const [exportStatus, setExportStatus] = useState<string | null>(null);

  const [importPath, setImportPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(
    null,
  );
  const [importStatus, setImportStatus] = useState<string | null>(null);

  // Recovery rotation state
  const [rotateModalOpen, setRotateModalOpen] = useState(false);
  const [rotateRecovery, setRotateRecovery] = useState<RecoveryInfo | null>(
    null,
  );
  const [rotatePassword, setRotatePassword] = useState("");
  const [rotateError, setRotateError] = useState<string | null>(null);
  const [rotateLoading, setRotateLoading] = useState(false);

  // Change-password state (re-keys vault; forces new recovery phrase)
  const [oldPwd, setOldPwd] = useState("");
  const [newPwd, setNewPwd] = useState("");
  const [confirmPwd, setConfirmPwd] = useState("");
  const [changePwdError, setChangePwdError] = useState<string | null>(null);
  const [changePwdLoading, setChangePwdLoading] = useState(false);

  useEffect(() => {
    loadAllSettings();
  }, []);

  const loadAllSettings = async () => {
    try {
      const pn = await getSetting("profile_name");
      if (pn) setProfileName(pn);

      const bc = await getSetting("brand_company");
      if (bc) setBrandCompany(bc);

      const th = await getSetting("appearance.theme");
      if (th === "light" || th === "dark") {
        setTheme(th);
        setColorScheme(th);
      }

      const ac = await getSetting("appearance.accent_color");
      if (ac) setAccentColor(ac);

      const w = await getSetting("appearance.dashboard_widgets");
      if (w) setWidgets({ ...DEFAULT_WIDGETS, ...JSON.parse(w) });

      const n = await getSetting("appearance.nav_items");
      if (n) setNavItems({ ...DEFAULT_NAV, ...JSON.parse(n) });

      const ou = await getSetting("ollama_url");
      if (ou) setOllamaUrl(ou);

      const em = await getSetting("email.smtp_host");
      if (em) setSmtpHost(em);
      const ep = await getSetting("email.smtp_port");
      if (ep) setSmtpPort(ep);
      const eu = await getSetting("email.smtp_user");
      if (eu) setSmtpUser(eu);
      const epw = await getSetting("email.smtp_password");
      if (epw) setSmtpPassword(epw);
      const et = await getSetting("email.smtp_tls");
      if (et) setSmtpTls(et === "true");
      const ef = await getSetting("email.smtp_from");
      if (ef) setSmtpFrom(ef);

      const fbd = await getSetting("email.followup_feedback_days");
      if (fbd) setFeedbackDays(fbd);
      const rtd = await getSetting("email.followup_retest_days");
      if (rtd) setRetestDays(rtd);
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  };

  const save = async (key: string, value: string) => {
    try {
      await setSetting(key, value);
    } catch (e) {
      console.error(`Failed to save setting ${key}:`, e);
    }
  };

  const handleTestOllama = async () => {
    setTestResult(null);
    setTestingOllama(true);
    try {
      const ok = await testOllamaConnection(ollamaUrl);
      setTestResult({
        success: ok,
        message: ok ? "Ollama is reachable." : "Ollama returned an error.",
      });
      await save("ollama_url", ollamaUrl);
    } catch (e) {
      setTestResult({ success: false, message: String(e) });
    } finally {
      setTestingOllama(false);
    }
  };

  const handleTestSmtp = async () => {
    setSmtpTestResult(null);
    setTestingSmtp(true);
    try {
      await testSmtpConnection();
      setSmtpTestResult("SMTP connection successful.");
    } catch (e) {
      setSmtpTestResult(`SMTP test failed: ${String(e)}`);
    } finally {
      setTestingSmtp(false);
    }
  };

  const handleRotate = async () => {
    setRotateError(null);
    setRotateLoading(true);
    try {
      const info = await rotateRecoveryPhrase(rotatePassword);
      setRotateRecovery(info);
      setRotateModalOpen(true);
    } catch (e) {
      setRotateError(String(e));
    } finally {
      setRotateLoading(false);
    }
  };

  const handleChangePassword = async () => {
    setChangePwdError(null);
    if (newPwd.length < 8) {
      setChangePwdError("New password must be at least 8 characters.");
      return;
    }
    if (newPwd !== confirmPwd) {
      setChangePwdError("New passwords do not match.");
      return;
    }
    setChangePwdLoading(true);
    try {
      const info = await changeMasterPassword(oldPwd, newPwd);
      // Reuse the recovery modal: is_rotation=true shows the right title.
      setRotateRecovery(info);
      setRotateModalOpen(true);
      setOldPwd("");
      setNewPwd("");
      setConfirmPwd("");
    } catch (e) {
      setChangePwdError(String(e));
    } finally {
      setChangePwdLoading(false);
    }
  };

  return (
    <>
      <Tabs defaultValue="profile">
        <Tabs.List>
          <Tabs.Tab value="profile">Profile</Tabs.Tab>
          <Tabs.Tab value="brand">Brand</Tabs.Tab>
          <Tabs.Tab value="appearance">Appearance</Tabs.Tab>
          <Tabs.Tab value="extensions">Extensions</Tabs.Tab>
          <Tabs.Tab value="ai">AI</Tabs.Tab>
          <Tabs.Tab value="email">Email</Tabs.Tab>
          <Tabs.Tab value="security">Security</Tabs.Tab>
          <Tabs.Tab value="data">Data</Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="profile" pt="md">
          <Stack>
            <TextInput
              label="Display Name"
              description="Your name or alias for the vault."
              value={profileName}
              onChange={(event) => setProfileName(event.currentTarget.value)}
              onBlur={() => save("profile_name", profileName)}
            />
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="brand" pt="md">
          <Stack>
            <TextInput
              label="Company / Organisation"
              description="Displayed in generated reports."
              value={brandCompany}
              onChange={(event) => setBrandCompany(event.currentTarget.value)}
              onBlur={() => save("brand_company", brandCompany)}
            />
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="appearance" pt="md">
          <Stack gap="lg">
            <Select
              label="Color Scheme"
              description="Choose your preferred theme."
              data={[
                { value: "light", label: "Light" },
                { value: "dark", label: "Dark" },
              ]}
              value={theme}
              onChange={(value) => {
                if (value === "light" || value === "dark") {
                  setTheme(value);
                  setColorScheme(value);
                  save("appearance.theme", value);
                }
              }}
            />

            <Stack gap="xs">
              <Text fw={600}>Accent Color</Text>
              <Group gap="xs">
                {PREDEFINED_COLORS.map((c) => (
                  <Button
                    key={c.hex}
                    size="xs"
                    style={{
                      backgroundColor: c.hex,
                      color: "#fff",
                      border:
                        accentColor === c.hex
                          ? "2px solid #000"
                          : "2px solid transparent",
                    }}
                    onClick={() => {
                      setAccentColor(c.hex);
                      save("appearance.accent_color", c.hex);
                    }}
                  >
                    {c.name}
                  </Button>
                ))}
              </Group>
              <ColorPicker
                format="hex"
                value={accentColor}
                onChange={(v) => {
                  setAccentColor(v);
                  save("appearance.accent_color", v);
                }}
              />
            </Stack>

            <Stack gap="xs">
              <Text fw={600}>Dashboard Widgets</Text>
              {Object.entries(widgets).map(([key, value]) => (
                <Switch
                  key={key}
                  label={key
                    .replace(/_/g, " ")
                    .replace(/\b\w/g, (l) => l.toUpperCase())}
                  checked={value}
                  onChange={(event) => {
                    const next = {
                      ...widgets,
                      [key]: event.currentTarget.checked,
                    };
                    setWidgets(next);
                    save("appearance.dashboard_widgets", JSON.stringify(next));
                  }}
                />
              ))}
            </Stack>

            <Stack gap="xs">
              <Text fw={600}>Navigation Items</Text>
              {Object.entries(navItems).map(([key, value]) => (
                <Switch
                  key={key}
                  label={key
                    .replace(/_/g, " ")
                    .replace(/\b\w/g, (l) => l.toUpperCase())}
                  checked={value}
                  disabled={key === "dashboard" || key === "settings"}
                  onChange={(event) => {
                    const next = {
                      ...navItems,
                      [key]: event.currentTarget.checked,
                    };
                    setNavItems(next);
                    save("appearance.nav_items", JSON.stringify(next));
                  }}
                />
              ))}
            </Stack>
          </Stack>
        </Tabs.Panel>


        <Tabs.Panel value="extensions" pt="md">
          <Stack>
            <Text size="lg" fw={700}>Extensions</Text>
            <Text size="sm" c="dimmed">
              Enable or disable feature extensions. Disabled extensions will be hidden from the navigation.
            </Text>
            <ExtensionToggleList />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="ai" pt="md">
          <Stack>
            <TextInput
              label="Ollama URL"
              description="URL of your local Ollama instance."
              value={ollamaUrl}
              onChange={(event) => setOllamaUrl(event.currentTarget.value)}
            />
            <Group>
              <Button onClick={handleTestOllama} loading={testingOllama}>
                Test Connection
              </Button>
            </Group>
            {testResult && (
              <Alert
                color={testResult.success ? "green" : "red"}
                variant="light"
              >
                {testResult.message}
              </Alert>
            )}
            <Text c="dimmed" size="xs">
              Settings are saved automatically on blur.
            </Text>
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="email" pt="md">
          <Stack>
            <Text fw={600}>SMTP Settings</Text>
            <TextInput
              label="SMTP Host"
              placeholder="smtp.example.com"
              value={smtpHost}
              onChange={(event) => setSmtpHost(event.currentTarget.value)}
              onBlur={() => save("email.smtp_host", smtpHost)}
            />
            <TextInput
              label="SMTP Port"
              placeholder="587"
              value={smtpPort}
              onChange={(event) => setSmtpPort(event.currentTarget.value)}
              onBlur={() => save("email.smtp_port", smtpPort)}
            />
            <TextInput
              label="Username"
              placeholder="user@example.com"
              value={smtpUser}
              onChange={(event) => setSmtpUser(event.currentTarget.value)}
              onBlur={() => save("email.smtp_user", smtpUser)}
            />
            <PasswordInput
              label="Password"
              placeholder="SMTP password"
              value={smtpPassword}
              onChange={(event) => setSmtpPassword(event.currentTarget.value)}
              onBlur={() => save("email.smtp_password", smtpPassword)}
            />
            <Switch
              label="Use TLS"
              checked={smtpTls}
              onChange={(event) => {
                const v = event.currentTarget.checked;
                setSmtpTls(v);
                save("email.smtp_tls", v ? "true" : "false");
              }}
            />
            <TextInput
              label="From Address"
              placeholder="notifications@example.com"
              value={smtpFrom}
              onChange={(event) => setSmtpFrom(event.currentTarget.value)}
              onBlur={() => save("email.smtp_from", smtpFrom)}
            />
            <Group>
              <Button onClick={handleTestSmtp} loading={testingSmtp}>
                Test SMTP Connection
              </Button>
            </Group>
            {smtpTestResult && (
              <Alert
                color={
                  smtpTestResult.startsWith("SMTP connection") ? "green" : "red"
                }
                variant="light"
              >
                {smtpTestResult}
              </Alert>
            )}
            <Text fw={600} mt="md">
              Follow-up Reminders
            </Text>
            <Text size="sm" c="dimmed">
              Days after engagement completion to prompt follow-up actions.
            </Text>
            <TextInput
              label="Feedback reminder (days)"
              type="number"
              value={feedbackDays}
              onChange={(event) => setFeedbackDays(event.currentTarget.value)}
              onBlur={() => save("email.followup_feedback_days", feedbackDays)}
            />
            <TextInput
              label="Retest reminder (days)"
              type="number"
              value={retestDays}
              onChange={(event) => setRetestDays(event.currentTarget.value)}
              onBlur={() => save("email.followup_retest_days", retestDays)}
            />
            <Text c="dimmed" size="xs">
              Settings are saved automatically on blur.
            </Text>
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="security" pt="md">
          <Stack>
            <Text fw={600}>Recovery Phrase</Text>
            <Text c="dimmed">
              Generate a new recovery phrase. This invalidates any previously
              written phrase.
            </Text>
            <PasswordInput
              label="Current Password"
              description="Required to verify your identity."
              placeholder="Enter current password..."
              value={rotatePassword}
              onChange={(e) => setRotatePassword(e.currentTarget.value)}
            />
            <Button onClick={handleRotate} loading={rotateLoading}>
              Generate New Recovery Phrase
            </Button>
            {rotateError && (
              <Alert color="red" variant="light">
                {rotateError}
              </Alert>
            )}

            <Text fw={600} mt="lg">
              Change Master Password
            </Text>
            <Text c="dimmed">
              Re-keys the vault with a new password. The current recovery phrase
              is invalidated and a new one will be generated; you must write it
              down and validate it before the change is complete.
            </Text>
            <PasswordInput
              label="Current Password"
              placeholder="Enter current password..."
              value={oldPwd}
              onChange={(e) => setOldPwd(e.currentTarget.value)}
            />
            <PasswordInput
              label="New Password"
              description="Minimum 8 characters."
              placeholder="Enter new password..."
              value={newPwd}
              onChange={(e) => setNewPwd(e.currentTarget.value)}
            />
            <PasswordInput
              label="Confirm New Password"
              placeholder="Re-enter new password..."
              value={confirmPwd}
              onChange={(e) => setConfirmPwd(e.currentTarget.value)}
            />
            <Button
              onClick={handleChangePassword}
              loading={changePwdLoading}
              disabled={!oldPwd || !newPwd || !confirmPwd}
            >
              Change Password
            </Button>
            {changePwdError && (
              <Alert color="red" variant="light">
                {changePwdError}
              </Alert>
            )}
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="data" pt="md">
          <Stack gap="lg">
            <Stack>
              <Text fw={600}>Export</Text>
              <Text c="dimmed">
                Export all vault data to a JSON file for backup.
              </Text>
              <TextInput
                label="Export file path"
                placeholder="/home/username/vault-export.json"
                value={exportPath}
                onChange={(e) => setExportPath(e.currentTarget.value)}
              />
              <Button
                onClick={async () => {
                  if (!exportPath.trim()) return;
                  setExporting(true);
                  try {
                    const result = await exportVaultJson(exportPath.trim());
                    setExportStatus(
                      `Exported ${result.clients} clients, ${result.engagements} engagements, ${result.findings} findings, ${result.documents} documents, ${result.invoices} invoices to ${result.file_path}`,
                    );
                  } catch (e) {
                    setExportStatus(`Export failed: ${String(e)}`);
                  } finally {
                    setExporting(false);
                  }
                }}
                loading={exporting}
                disabled={!exportPath.trim()}
              >
                Export Vault JSON
              </Button>
              {exportStatus && (
                <Alert color="blue" variant="light">
                  {exportStatus}
                </Alert>
              )}
            </Stack>

            <Stack>
              <Text fw={600}>Import</Text>
              <Text c="dimmed">
                Import data from a previously exported JSON file.
              </Text>
              <TextInput
                label="Import file path"
                placeholder="/home/username/vault-export.json"
                value={importPath}
                onChange={(e) => setImportPath(e.currentTarget.value)}
              />
              <Button
                onClick={async () => {
                  if (!importPath.trim()) return;
                  try {
                    const preview = await previewImport(importPath.trim());
                    setImportPreview(preview);
                  } catch (e) {
                    setImportStatus(`Preview failed: ${String(e)}`);
                  }
                }}
                disabled={!importPath.trim()}
                variant="default"
              >
                Preview Import
              </Button>
              {importPreview && importPreview.conflicts.length > 0 && (
                <Alert color="yellow" variant="light">
                  {importPreview.conflicts.length} conflict(s) detected:{" "}
                  {importPreview.conflicts.map((c) => c.import_name).join(", ")}
                </Alert>
              )}
              <Button
                onClick={async () => {
                  if (!importPath.trim()) return;
                  setImporting(true);
                  try {
                    const resolutions: ConflictResolution[] =
                      importPreview?.conflicts.map((c) => ({
                        reference_key: c.import_name,
                        action: "rename",
                      })) || [];
                    const result = await executeImport(
                      importPath.trim(),
                      resolutions,
                    );
                    setImportStatus(
                      `Imported ${Object.values(result.imported).reduce((a, b) => a + b, 0)} entities.`,
                    );
                  } catch (e) {
                    setImportStatus(`Import failed: ${String(e)}`);
                  } finally {
                    setImporting(false);
                  }
                }}
                loading={importing}
                disabled={
                  !importPath.trim() ||
                  !!(importPreview && importPreview.compatible === false)
                }
              >
                Execute Import
              </Button>
              {importStatus && (
                <Alert color="blue" variant="light">
                  {importStatus}
                </Alert>
              )}
            </Stack>
          </Stack>
        </Tabs.Panel>
      </Tabs>

      {rotateRecovery && (
        <RecoveryPhraseModal
          opened={rotateModalOpen}
          recovery={rotateRecovery}
          onSuccess={() => {
            setRotateModalOpen(false);
            setRotateRecovery(null);
            setRotatePassword("");
          }}
          onClose={() => {
            setRotateModalOpen(false);
            setRotateRecovery(null);
            setRotatePassword("");
          }}
          allowClose={true}
        />
      )}
    </>
  );
}
