import { invoke } from "@tauri-apps/api/core";

export interface CalendarEvent {
  id: number;
  client_id: number;
  client_name: string;
  name: string;
  start_date: string | null;
  end_date: string | null;
  status: string;
}

export interface Reminder {
  reminder_key: string;
  reminder_type: string;
  entity_id: number;
  entity_name: string;
  client_name: string;
  due_date: string;
  days_until: number;
  urgency: "overdue" | "today" | "upcoming";
}

export async function listCalendarEvents(): Promise<CalendarEvent[]> {
  return invoke<CalendarEvent[]>("list_calendar_events");
}

export async function getActiveReminders(): Promise<Reminder[]> {
  return invoke<Reminder[]>("get_active_reminders");
}

export async function dismissReminder(reminderKey: string): Promise<void> {
  return invoke("dismiss_reminder", { reminderKey });
}
