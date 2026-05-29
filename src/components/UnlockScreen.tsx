import { useState, useEffect } from "react";
import {
  Alert,
  Button,
  Center,
  LoadingOverlay,
  Paper,
  PasswordInput,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { isVaultInitialized, createVault, unlockVault } from "../api/auth";

interface Props {
  onUnlocked: () => void;
}

export function UnlockScreen({ onUnlocked }: Props) {
  const [initialized, setInitialized] = useState<boolean | null>(null);
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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
      await createVault(password);
      onUnlocked();
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
      await unlockVault(password);
      onUnlocked();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Center h="100vh" p="md">
      <Paper p="xl" shadow="md" radius="md" withBorder w={420}>
        <Stack>
          <Title order={3} ta="center">
            SecuritySmith
          </Title>
          <Text c="dimmed" ta="center" size="sm">
            {initialized
              ? "Enter your master password to unlock the vault."
              : "Create a master password for your secure vault."}
          </Text>
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
          {error && (
            <Alert color="red" variant="light">
              {error}
            </Alert>
          )}
          <Button
            loading={loading}
            onClick={initialized ? handleUnlock : handleCreate}
          >
            {initialized ? "Unlock" : "Create Vault"}
          </Button>
        </Stack>
      </Paper>
    </Center>
  );
}
