import { Center, Stack, Text, Title } from "@mantine/core";

export function Dashboard() {
  return (
    <Center h="calc(100vh - 120px)">
      <Stack align="center" gap="sm">
        <Title order={2}>Dashboard</Title>
        <Text c="dimmed" size="lg">
          Your vault is unlocked and ready.
        </Text>
        <Text c="dimmed" size="sm">
          Start adding security data to see it here.
        </Text>
      </Stack>
    </Center>
  );
}
