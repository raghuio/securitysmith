import { invoke } from "@tauri-apps/api/core";

export interface RecoveryInfo {
  phrase: string;
  positions: number[];
  is_rotation: boolean;
}

export interface ValidationResult {
  success: boolean;
  new_phrase: string | null;
  new_positions: number[] | null;
}

export async function isVaultInitialized(): Promise<boolean> {
  return invoke<boolean>("is_vault_initialized");
}

export async function createVault(password: string): Promise<RecoveryInfo> {
  return invoke<RecoveryInfo>("create_vault", { password });
}

export async function unlockVault(password: string): Promise<boolean> {
  // Returns `true` when the vault is missing a recovery envelope
  // (legacy vault from before PROP-002). Frontend should call
  // `rotateRecoveryPhrase` to bootstrap recovery in that case.
  return invoke<boolean>("unlock_vault", { password });
}

export async function validateRecoveryWords(
  phrase: string,
  positions: number[],
  words: string[],
): Promise<ValidationResult> {
  return invoke<ValidationResult>("validate_recovery_words", {
    phrase,
    positions,
    words,
  });
}

export async function recoverVault(phrase: string): Promise<void> {
  return invoke<void>("recover_vault", { phrase });
}

export async function rotateRecoveryPhrase(
  password: string,
): Promise<RecoveryInfo> {
  return invoke<RecoveryInfo>("rotate_recovery_phrase", { password });
}

export async function changeMasterPassword(
  oldPassword: string,
  newPassword: string,
): Promise<RecoveryInfo> {
  return invoke<RecoveryInfo>("change_master_password", {
    oldPassword,
    newPassword,
  });
}
