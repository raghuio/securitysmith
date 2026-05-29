import { useState, useEffect } from "react";
import {
  Alert,
  Button,
  Group,
  Select,
  Stack,
  Tabs,
  Text,
  TextInput,
} from "@mantine/core";
import { useMantineColorScheme } from "@mantine/core";
import { getSetting, setSetting, testOllamaConnection } from "../api/settings";

export function SettingsPage() {
  const { setColorScheme } = useMantineColorScheme();

  const [profileName, setProfileName] = useState("");
  const [brandCompany, setBrandCompany] = useState("");
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");

  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);
  const [testingOllama, setTestingOllama] = useState(false);

  useEffect(() => {
    loadAllSettings();
  }, []);

  const loadAllSettings = async () => {
    try {
      const pn = await getSetting("profile_name");
      if (pn) setProfileName(pn);

      const bc = await getSetting("brand_company");
      if (bc) setBrandCompany(bc);

      const th = await getSetting("theme");
      if (th === "light" || th === "dark") {
        setTheme(th);
        setColorScheme(th);
      }

      const ou = await getSetting("ollama_url");
      if (ou) setOllamaUrl(ou);
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

  return (
    <Tabs defaultValue="profile">
      <Tabs.List>
        <Tabs.Tab value="profile">Profile</Tabs.Tab>
        <Tabs.Tab value="brand">Brand</Tabs.Tab>
        <Tabs.Tab value="appearance">Appearance</Tabs.Tab>
        <Tabs.Tab value="ai">AI</Tabs.Tab>
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
        <Stack>
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
                save("theme", value);
              }
            }}
          />
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
            <Alert color={testResult.success ? "green" : "red"} variant="light">
              {testResult.message}
            </Alert>
          )}
          <Text c="dimmed" size="xs">
            Settings are saved automatically on blur.
          </Text>
        </Stack>
      </Tabs.Panel>
    </Tabs>
  );
}
