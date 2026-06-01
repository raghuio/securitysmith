import { notifications } from "@mantine/notifications";

export function useNotify() {
  const success = (message: string, title = "Success") => {
    notifications.show({
      title,
      message,
      color: "green",
    });
  };

  const error = (message: string, title = "Error") => {
    notifications.show({
      title,
      message,
      color: "red",
    });
  };

  const info = (message: string, title = "Info") => {
    notifications.show({
      title,
      message,
      color: "blue",
    });
  };

  return { success, error, info };
}
