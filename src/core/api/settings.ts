import { invoke } from "@tauri-apps/api/core";

export interface SettingEntry {
  key: string;
  value: string;
}

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export async function listSettings(): Promise<SettingEntry[]> {
  return invoke<SettingEntry[]>("list_settings");
}

export async function getBootTheme(): Promise<string> {
  return invoke<string>("get_boot_theme");
}

export async function testOllamaConnection(url: string): Promise<boolean> {
  return invoke<boolean>("test_ollama_connection", { url });
}
