import { useState, useEffect } from "react";
import {
  Alert,
  Anchor,
  Button,
  Center,
  LoadingOverlay,
  Paper,
  PasswordInput,
  Stack,
  Text,
  Textarea,
  Title,
} from "@mantine/core";
import {
  isVaultInitialized,
  createVault,
  unlockVault,
  recoverVault,
  rotateRecoveryPhrase,
} from "../api/auth";
import type { RecoveryInfo } from "../api/auth";
import { RecoveryPhraseModal } from "./RecoveryPhraseModal";

interface Props {
  onUnlocked: () => void;
}

export function UnlockScreen({ onUnlocked }: Props) {
  const [initialized, setInitialized] = useState<boolean | null>(null);
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [recoveryPhrase, setRecoveryPhrase] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<"password" | "recovery">("password");
  const [pendingRecovery, setPendingRecovery] = useState<RecoveryInfo | null>(
    null,
  );
  // True when the recovery modal is for an upgraded vault (not a fresh
  // creation). Allows the user to dismiss and configure later from
  // Settings → Security.
  const [pendingRecoveryIsOptional, setPendingRecoveryIsOptional] =
    useState(false);

  useEffect(() => {
    isVaultInitialized()
      .then(setInitialized)
      .catch(() => {
        setInitialized(false);
        setError("Failed to check vault status");
      });
  }, []);

  if (initialized === null) {
    return (
      <Center h="100vh">
        <LoadingOverlay visible />
      </Center>
    );
  }

  const handleCreate = async () => {
    setError(null);
    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }
    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }
    setLoading(true);
    try {
      const info = await createVault(password);
      setPendingRecovery(info);
      setPendingRecoveryIsOptional(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleUnlock = async () => {
    setError(null);
    setLoading(true);
    try {
      const recoverySetupRequired = await unlockVault(password);
      if (recoverySetupRequired) {
        // Legacy vault (created before PROP-002): bootstrap a recovery
        // phrase. The user can dismiss the modal and configure it later
        // from Settings → Security.
        const info = await rotateRecoveryPhrase(password);
        setPendingRecovery(info);
        setPendingRecoveryIsOptional(true);
        // Do NOT call onUnlocked() yet — the modal must be shown.
        return;
      }
      onUnlocked();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleRecover = async () => {
    setError(null);
    setLoading(true);
    try {
      await recoverVault(recoveryPhrase.trim());
      onUnlocked();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (loading) return;
    if (!initialized) {
      handleCreate();
    } else if (mode === "password") {
      handleUnlock();
    } else {
      handleRecover();
    }
  };

  return (
    <>
      <Center h="100vh" p="md">
        <Paper p="xl" shadow="md" radius="md" withBorder w={420}>
          <form onSubmit={handleFormSubmit}>
            <Stack>
              <Title order={3} ta="center">
                SecuritySmith
              </Title>
              <Text c="dimmed" ta="center" size="sm">
                {initialized
                  ? "Enter your master password to unlock the vault."
                  : "Create a master password for your secure vault."}
              </Text>

              {mode === "password" ? (
                <>
                  <PasswordInput
                    label="Master Password"
                    placeholder="Enter password…"
                    value={password}
                    onChange={(event) => setPassword(event.currentTarget.value)}
                    autoFocus
                  />
                  {!initialized && (
                    <PasswordInput
                      label="Confirm Password"
                      placeholder="Confirm password…"
                      value={confirmPassword}
                      onChange={(event) =>
                        setConfirmPassword(event.currentTarget.value)
                      }
                    />
                  )}
                </>
              ) : (
                <Textarea
                  label="Recovery Phrase"
                  description="Enter your 12-word recovery phrase."
                  placeholder="word1 word2 word3 ..."
                  value={recoveryPhrase}
                  onChange={(event) =>
                    setRecoveryPhrase(event.currentTarget.value)
                  }
                  minRows={3}
                  autoFocus
                />
              )}

              {error && (
                <Alert color="red" variant="light">
                  {error}
                </Alert>
              )}

              <Button type="submit" loading={loading}>
                {initialized
                  ? mode === "password"
                    ? "Unlock"
                    : "Recover Vault"
                  : "Create Vault"}
              </Button>

              {initialized && (
                <Anchor
                  component="button"
                  type="button"
                  size="sm"
                  ta="center"
                  onClick={() => {
                    setMode(mode === "password" ? "recovery" : "password");
                    setError(null);
                  }}
                >
                  {mode === "password"
                    ? "Forgot password? Use recovery phrase"
                    : "Back to password unlock"}
                </Anchor>
              )}

              {!initialized && (
                <Text c="dimmed" size="xs" ta="center">
                  If you lose this password, your data cannot be recovered
                  unless you save your recovery phrase.
                </Text>
              )}
            </Stack>
          </form>
        </Paper>
      </Center>

      {pendingRecovery && (
        <RecoveryPhraseModal
          opened={true}
          recovery={pendingRecovery}
          onSuccess={() => {
            setPendingRecovery(null);
            onUnlocked();
          }}
          onClose={() => {
            setPendingRecovery(null);
            // For upgraded vaults the user can dismiss the modal; the
            // vault is already unlocked, so proceed.
            onUnlocked();
          }}
          allowClose={pendingRecoveryIsOptional}
        />
      )}
    </>
  );
}
