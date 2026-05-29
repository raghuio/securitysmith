import { invoke } from "@tauri-apps/api/core";

export async function isVaultInitialized(): Promise<boolean> {
  return invoke<boolean>("is_vault_initialized");
}

export async function createVault(password: string): Promise<void> {
  return invoke<void>("create_vault", { password });
}

export async function unlockVault(password: string): Promise<void> {
  return invoke<void>("unlock_vault", { password });
}
