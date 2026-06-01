import { invoke } from "@tauri-apps/api/core";

export interface Notification {
  id: string;
  category: string;
  title: string;
  description: string;
  entity_type: string;
  entity_id: number;
  timestamp: number;
  is_dismissed: boolean;
}

export async function getNotifications(): Promise<Notification[]> {
  return invoke<Notification[]>("get_notifications");
}

export async function dismissNotification(key: string): Promise<void> {
  return invoke<void>("dismiss_notification", { key });
}

export async function getNotificationCount(): Promise<number> {
  return invoke<number>("get_notification_count");
}
