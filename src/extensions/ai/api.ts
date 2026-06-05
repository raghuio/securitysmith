import { invoke } from "@tauri-apps/api/core";

export interface AiChatResponse {
  message: string;
}

export async function aiChat(
  prompt: string,
  context?: string,
): Promise<AiChatResponse> {
  return invoke<AiChatResponse>("ai_chat", {
    prompt,
    context: context ?? null,
  });
}

export async function aiApproveToolCall(callId: string): Promise<string> {
  return invoke<string>("ai_approve_tool_call", { callId });
}

export async function aiRejectToolCall(callId: string): Promise<void> {
  return invoke("ai_reject_tool_call", { callId });
}

export async function aiCancel(): Promise<void> {
  return invoke("ai_cancel");
}
