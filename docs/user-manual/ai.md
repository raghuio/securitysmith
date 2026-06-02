# AI Assistant (Ollama)

## Setup

1. Install [Ollama](https://ollama.com) on your machine.
2. Pull a model: `ollama pull llama3`
3. Start Ollama (it runs at `http://localhost:11434` by default).
4. In SecuritySmith, go to **Settings → AI** and verify the URL.
5. Click **Test Connection**.

## Minibuffer

The AI Minibuffer is a persistent bottom-bar chat interface.

- Open/close with `Ctrl+;`
- Ask questions about your current screen context
- Draft content: "Draft an executive summary for the Acme engagement"
- Execute commands: "Create a new client named Acme Corp"

## Tool Calls

When the AI suggests an action (e.g., creating a client or finding), you see a confirmation dialog with the exact action before anything changes. You must **approve** or **reject** every tool call.

No database mutation happens without your explicit approval.

## Privacy

All AI processing happens **locally** on your machine. No data leaves your device.

## Session History

Conversation history is maintained per session and cleared when you restart the app. Nothing is stored permanently.
