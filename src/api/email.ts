import { invoke } from "@tauri-apps/api/core";

export interface FollowUpReminder {
  engagement_id: number;
  engagement_name: string;
  client_name: string;
  reminder_type: "feedback" | "retest";
  due_date: string;
  days_overdue: number;
}

export async function sendEmail(
  to: string,
  subject: string,
  body: string,
  attachments: string[],
  clientId?: number,
  engagementId?: number,
): Promise<void> {
  return invoke("send_email", {
    to,
    subject,
    body,
    attachments,
    clientId,
    engagementId,
  });
}

export async function testSmtpConnection(): Promise<boolean> {
  return invoke<boolean>("test_smtp_connection");
}

export async function getFollowUpReminders(): Promise<FollowUpReminder[]> {
  return invoke<FollowUpReminder[]>("get_follow_up_reminders");
}

export async function dismissReminder(reminderKey: string): Promise<void> {
  return invoke("dismiss_reminder", { reminderKey });
}
