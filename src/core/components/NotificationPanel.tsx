import { useEffect, useState } from "react";
import {
  ActionIcon,
  Badge,
  Button,
  Card,
  Drawer,
  Group,
  ScrollArea,
  Stack,
  Text,
} from "@mantine/core";
import { IconBell } from "@tabler/icons-react";
import {
  getNotifications,
  dismissNotification,
  getNotificationCount,
  type Notification,
} from "../api/notifications";

export function NotificationBell() {
  const [opened, setOpened] = useState(false);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [count, setCount] = useState(0);

  const load = async () => {
    try {
      const [data, c] = await Promise.all([
        getNotifications(),
        getNotificationCount(),
      ]);
      setNotifications(data);
      setCount(c);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
    const interval = setInterval(load, 60000);
    return () => clearInterval(interval);
  }, []);

  const handleDismiss = async (key: string) => {
    try {
      await dismissNotification(key);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDismissAll = async () => {
    try {
      await Promise.all(notifications.map((n) => dismissNotification(n.id)));
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <>
      <ActionIcon
        variant="light"
        aria-label="Notifications"
        onClick={() => {
          setOpened(true);
          load();
        }}
        pos="relative"
      >
        <IconBell size={18} />
        {count > 0 && (
          <Badge
            size="xs"
            variant="filled"
            color="red"
            pos="absolute"
            top={-4}
            right={-4}
            style={{ padding: 0, minWidth: 18, height: 18 }}
          >
            {count}
          </Badge>
        )}
      </ActionIcon>

      <Drawer
        opened={opened}
        onClose={() => setOpened(false)}
        title="Notifications"
        position="right"
        size="md"
      >
        <Stack gap="sm">
          {notifications.length > 0 && (
            <Group justify="flex-end">
              <Button variant="subtle" size="xs" onClick={handleDismissAll}>
                Mark all read
              </Button>
            </Group>
          )}
          <ScrollArea h="calc(100vh - 120px)">
            <Stack gap="sm">
              {notifications.map((n) => (
                <Card key={n.id} withBorder padding="sm" radius="md">
                  <Group justify="space-between">
                    <Stack gap={2}>
                      <Text size="sm" fw={600}>
                        {n.title}
                      </Text>
                      <Text size="xs" c="dimmed">
                        {n.description}
                      </Text>
                    </Stack>
                    <Button
                      variant="subtle"
                      size="xs"
                      onClick={() => handleDismiss(n.id)}
                    >
                      Dismiss
                    </Button>
                  </Group>
                </Card>
              ))}
              {notifications.length === 0 && (
                <Text c="dimmed" ta="center">
                  No notifications
                </Text>
              )}
            </Stack>
          </ScrollArea>
        </Stack>
      </Drawer>
    </>
  );
}
