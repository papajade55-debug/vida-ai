import { useCallback, useRef } from "react";
import { api, onStreamEvent } from "@/src/lib/tauri";
import type { StreamEvent } from "@/src/lib/tauri";
import { useStore } from "@/src/stores/store";

export function useStreamCompletion() {
  const unlistenRef = useRef<(() => void) | null>(null);

  const streamingMessageId = useStore((s) => s.streamingMessageId);
  const currentSessionId = useStore((s) => s.currentSessionId);
  const addMessage = useStore((s) => s.addMessage);
  const startStreaming = useStore((s) => s.startStreaming);
  const appendToken = useStore((s) => s.appendToken);
  const finishStreaming = useStore((s) => s.finishStreaming);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!currentSessionId) return;

      const sessionId = currentSessionId;

      // Add user message to store
      const userMessage = {
        id: crypto.randomUUID(),
        session_id: sessionId,
        role: "user" as const,
        content,
        token_count: null,
        created_at: new Date().toISOString(),
        agent_id: null,
        agent_name: null,
        agent_color: null,
      };
      addMessage(sessionId, userMessage);

      // Create placeholder assistant message and start streaming
      const assistantId = crypto.randomUUID();
      const assistantMessage = {
        id: assistantId,
        session_id: sessionId,
        role: "assistant" as const,
        content: "",
        token_count: null,
        created_at: new Date().toISOString(),
        agent_id: null,
        agent_name: null,
        agent_color: null,
      };
      addMessage(sessionId, assistantMessage);
      startStreaming(assistantId);

      // Subscribe to stream events
      const unlisten = await onStreamEvent(
        sessionId,
        (event: StreamEvent) => {
          if (event === "Done") {
            finishStreaming();
            unlistenRef.current?.();
            unlistenRef.current = null;
          } else if ("Token" in event) {
            appendToken(event.Token.content);
          } else if ("Error" in event) {
            console.error("[stream] Error:", event.Error.error);
            finishStreaming();
            unlistenRef.current?.();
            unlistenRef.current = null;
          }
        }
      );
      unlistenRef.current = unlisten;

      // Trigger the Tauri command
      try {
        await api.streamCompletion(sessionId, content);
      } catch (err) {
        console.error("[stream] Failed to invoke stream_completion:", err);
        finishStreaming();
        unlistenRef.current?.();
        unlistenRef.current = null;
      }
    },
    [
      currentSessionId,
      addMessage,
      startStreaming,
      appendToken,
      finishStreaming,
    ]
  );

  return {
    sendMessage,
    isStreaming: streamingMessageId !== null,
  };
}
