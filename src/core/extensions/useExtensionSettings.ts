import { useState, useEffect, useCallback } from "react";
import { getSetting, setSetting } from "../api/settings";

export interface ExtensionSettings {
  [key: string]: boolean;
}

export function useExtensionSettings() {
  const [settings, setSettings] = useState<ExtensionSettings>({});
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    getSetting("extensions_enabled")
      .then((value) => {
        if (value) {
          try {
            setSettings(JSON.parse(value));
          } catch {
            setSettings({});
          }
        }
        setLoaded(true);
      })
      .catch(() => setLoaded(true));
  }, []);

  const toggleExtension = useCallback(async (id: string, enabled: boolean) => {
    const updated = { ...settings, [id]: enabled };
    setSettings(updated);
    await setSetting("extensions_enabled", JSON.stringify(updated));
  }, [settings]);

  const isEnabled = useCallback((id: string) => {
    return settings[id] ?? true; // Default to enabled
  }, [settings]);

  return { settings, loaded, toggleExtension, isEnabled };
}
