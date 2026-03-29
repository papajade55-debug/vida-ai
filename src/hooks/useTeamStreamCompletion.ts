import { useCallback, useRef, useState } from "react";
import { api, onTeamStreamEvent } from "@/src/lib/tauri";
import type { TeamStreamEvent, TeamMemberRow } from "@/src/lib/tauri";
import { useStore } from "@/src/stores/store";

export function useTeamStreamCompletion() {
  const unlistenRef = useRef<(() => void) | null>(null);
  const [isTeamStreaming, setIsTeamStreaming] = useState(false);

  const currentSessionId = useStore((s) => s.currentSessionId);
  const addMessage = useStore((s) => s.addMessage);
  const startTeamStreaming = useStore((s) => s.startTeamStreaming);
  const appendAgentToken = useStore((s) => s.appendAgentToken);
  const finishAgentStreaming = useStore((s) => s.finishAgentStreaming);
  const finishAllStreaming = useStore((s) => s.finishAllStreaming);

  const sendTeamMessage = useCallback(
    async (content: string, members: TeamMemberRow[]) => {
      if (!currentSessionId) return;

      const sessionId = currentSessionId;
      setIsTeamStreaming(true);

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

      // Create placeholder assistant messages (one per agent)
      const agentMessageIds: Record<string, string> = {};
      for (const member of members) {
        const msgId = crypto.randomUUID();
        agentMessageIds[member.id] = msgId;
        const assistantMessage = {
          id: msgId,
          session_id: sessionId,
          role: "assistant" as const,
          content: "",
          token_count: null,
          created_at: new Date().toISOString(),
          agent_id: member.id,
          agent_name: member.display_name || `${member.provider_id}/${member.model}`,
          agent_color: member.color,
        };
        addMessage(sessionId, assistantMessage);
      }

      // Start tracking per-agent streaming
      startTeamStreaming(members.map((m) => m.id));

      // Subscribe to team stream events
      const unlisten = await onTeamStreamEvent(
        sessionId,
        (event: TeamStreamEvent) => {
          if (event === "AllDone") {
            finishAllStreaming();
            setIsTeamStreaming(false);
            unlistenRef.current?.();
            unlistenRef.current = null;
          } else if ("AgentToken" in event) {
            appendAgentToken(event.AgentToken.agent_id, event.AgentToken.content);
          } else if ("AgentDone" in event) {
            finishAgentStreaming(event.AgentDone.agent_id);
          } else if ("AgentError" in event) {
            console.error(
              `[team-stream] Agent ${event.AgentError.agent_id} error:`,
              event.AgentError.error
            );
            finishAgentStreaming(event.AgentError.agent_id);
          }
        }
      );
      unlistenRef.current = unlisten;

      // Trigger the Tauri command
      try {
        await api.streamTeamCompletion(sessionId, content);
      } catch (err) {
        console.error("[team-stream] Failed to invoke stream_team_completion:", err);
        finishAllStreaming();
        setIsTeamStreaming(false);
        unlistenRef.current?.();
        unlistenRef.current = null;
      }
    },
    [
      currentSessionId,
      addMessage,
      startTeamStreaming,
      appendAgentToken,
      finishAgentStreaming,
      finishAllStreaming,
    ]
  );

  return { sendTeamMessage, isTeamStreaming };
}
