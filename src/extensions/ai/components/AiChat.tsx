import { useState, useEffect, useRef, useCallback } from "react";
import {
  Button,
  Group,
  Paper,
  ScrollArea,
  Stack,
  Text,
  TextInput,
  ActionIcon,
  Modal,
} from "@mantine/core";
import { IconSend, IconX, IconMessageCircle } from "@tabler/icons-react";
import { aiChat, aiApproveToolCall, aiRejectToolCall } from "../api";
import { listen } from "@tauri-apps/api/event";

interface Message {
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
}

interface ToolCallRequest {
  call_id: string;
  tool_name: string;
  arguments: Record<string, unknown>;
}

interface StreamChunk {
  chunk: string;
  accumulated: string;
}

export function AiChat() {
  const [open, setOpen] = useState(false);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [pendingTool, setPendingTool] = useState<ToolCallRequest | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const streamBufferRef = useRef("");

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === ";") {
        event.preventDefault();
        setOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTo({
        top: scrollRef.current.scrollHeight,
        behavior: "smooth",
      });
    }
  }, [messages, open]);

  const appendStreamChunk = useCallback((chunk: string) => {
    streamBufferRef.current += chunk;
    setMessages((prev) => {
      const last = prev[prev.length - 1];
      if (last && last.role === "assistant" && last.streaming) {
        const next = [...prev];
        next[next.length - 1] = {
          ...last,
          content: streamBufferRef.current,
        };
        return next;
      }
      return prev;
    });
  }, []);

  useEffect(() => {
    let unlistenTool: (() => void) | undefined;
    let unlistenChunk: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;

    const setup = async () => {
      unlistenTool = await listen<ToolCallRequest>(
        "ai_tool_call_request",
        (event) => {
          setPendingTool(event.payload);
        },
      );
      unlistenChunk = await listen<StreamChunk>("ai_stream_chunk", (event) => {
        appendStreamChunk(event.payload.chunk);
      });
      unlistenDone = await listen<{ full: string }>("ai_stream_done", () => {
        setLoading(false);
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.role === "assistant" && last.streaming) {
            const next = [...prev];
            next[next.length - 1] = {
              ...last,
              content: streamBufferRef.current,
              streaming: false,
            };
            return next;
          }
          return prev;
        });
      });
    };
    setup();
    return () => {
      if (unlistenTool) unlistenTool();
      if (unlistenChunk) unlistenChunk();
      if (unlistenDone) unlistenDone();
    };
  }, [appendStreamChunk]);

  const handleSend = async () => {
    const prompt = input.trim();
    if (!prompt) return;
    setInput("");
    streamBufferRef.current = "";
    setMessages((prev) => [
      ...prev,
      { role: "user", content: prompt },
      { role: "assistant", content: "", streaming: true },
    ]);
    setLoading(true);
    try {
      const result = await aiChat(prompt, "Dashboard");
      // If tool call or no streaming events arrived, fill the placeholder
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.role === "assistant" && last.streaming) {
          const next = [...prev];
          next[next.length - 1] = {
            ...last,
            content: result.message,
            streaming: false,
          };
          return next;
        }
        return prev;
      });
    } catch (e) {
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.role === "assistant" && last.streaming) {
          const next = [...prev];
          next[next.length - 1] = {
            ...last,
            content: `Error: ${String(e)}`,
            streaming: false,
          };
          return next;
        }
        return prev;
      });
    } finally {
      setLoading(false);
    }
  };

  const handleApproveTool = async () => {
    if (!pendingTool) return;
    try {
      const result = await aiApproveToolCall(pendingTool.call_id);
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: `Tool ${pendingTool.tool_name} approved. Result: ${result}`,
        },
      ]);
    } catch (e) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `Error: ${String(e)}` },
      ]);
    } finally {
      setPendingTool(null);
    }
  };

  const handleRejectTool = async () => {
    if (!pendingTool) return;
    try {
      await aiRejectToolCall(pendingTool.call_id);
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: `Tool ${pendingTool.tool_name} rejected.`,
        },
      ]);
    } catch (e) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `Error: ${String(e)}` },
      ]);
    } finally {
      setPendingTool(null);
    }
  };

  return (
    <>
      <Modal
        opened={!!pendingTool}
        onClose={() => setPendingTool(null)}
        title="AI Tool Call Request"
        centered
      >
        <Stack>
          <Text size="sm">
            The AI wants to execute: <strong>{pendingTool?.tool_name}</strong>
          </Text>
          <Text size="xs" c="dimmed">
            Arguments: {JSON.stringify(pendingTool?.arguments)}
          </Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={handleRejectTool}>
              Reject
            </Button>
            <Button onClick={handleApproveTool}>Approve</Button>
          </Group>
        </Stack>
      </Modal>

      {!open ? (
        <Paper
          withBorder
          p="xs"
          style={{
            position: "fixed",
            bottom: 0,
            left: 240,
            right: 0,
            zIndex: 100,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderTopLeftRadius: 8,
            borderTopRightRadius: 8,
            borderBottomLeftRadius: 0,
            borderBottomRightRadius: 0,
          }}
        >
          <Group gap="xs">
            <IconMessageCircle size={16} />
            <Text size="sm" c="dimmed">
              AI Minibuffer — Press Ctrl+; to expand
            </Text>
          </Group>
          <ActionIcon size="sm" variant="subtle" onClick={() => setOpen(true)}>
            <IconMessageCircle size={16} />
          </ActionIcon>
        </Paper>
      ) : (
        <Paper
          withBorder
          radius="md"
          p="sm"
          style={{
            position: "fixed",
            bottom: 0,
            left: 240,
            right: 0,
            height: 280,
            zIndex: 100,
            display: "flex",
            flexDirection: "column",
            borderTopLeftRadius: 8,
            borderTopRightRadius: 8,
            borderBottomLeftRadius: 0,
            borderBottomRightRadius: 0,
          }}
        >
          <Group justify="space-between" mb="xs">
            <Group gap="xs">
              <IconMessageCircle size={16} />
              <Text fw={600} size="sm">
                AI Minibuffer ({" "}
                {messages.filter((m) => m.role === "assistant").length}{" "}
                responses)
              </Text>
            </Group>
            <ActionIcon
              size="sm"
              variant="subtle"
              onClick={() => setOpen(false)}
            >
              <IconX size={14} />
            </ActionIcon>
          </Group>
          <ScrollArea style={{ flex: 1 }} mb="xs" ref={scrollRef}>
            <Stack gap="xs" pr="xs">
              {messages.map((m, i) => (
                <Paper
                  key={i}
                  p="xs"
                  withBorder
                  bg={m.role === "user" ? "gray.0" : "blue.0"}
                >
                  <Text size="xs" c="dimmed" fw={600} mb={2}>
                    {m.role === "user" ? "You" : "Ollama"}
                    {m.streaming && " …"}
                  </Text>
                  <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
                    {m.content}
                  </Text>
                </Paper>
              ))}
              {messages.length === 0 && (
                <Text size="sm" c="dimmed">
                  Ask the local Ollama model anything...
                </Text>
              )}
            </Stack>
          </ScrollArea>
          <Group gap="xs">
            <TextInput
              placeholder="Type a prompt... (Ctrl+; to toggle)"
              value={input}
              onChange={(e) => setInput(e.currentTarget.value)}
              style={{ flex: 1 }}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSend();
                }
              }}
            />
            <Button onClick={handleSend} loading={loading}>
              <IconSend size={16} />
            </Button>
          </Group>
        </Paper>
      )}
    </>
  );
}
