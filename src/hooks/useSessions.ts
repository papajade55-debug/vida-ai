import { useEffect, useCallback } from "react";
import { useStore } from "@/src/stores/store";
import { api } from "@/src/lib/tauri";

export function useSessions() {
  const sessions = useStore((s) => s.sessions);
  const currentSessionId = useStore((s) => s.currentSessionId);
  const setSessions = useStore((s) => s.setSessions);
  const setCurrentSession = useStore((s) => s.setCurrentSession);
  const addSession = useStore((s) => s.addSession);
  const removeSession = useStore((s) => s.removeSession);
  const setMessages = useStore((s) => s.setMessages);

  useEffect(() => {
    api.listSessions(50).then(setSessions).catch(console.error);
  }, [setSessions]);

  const selectSession = useCallback(
    async (id: string) => {
      setCurrentSession(id);
      try {
        const msgs = await api.getMessages(id);
        setMessages(id, msgs);
      } catch (e) {
        console.error("Failed to load messages:", e);
      }
    },
    [setCurrentSession, setMessages]
  );

  const createSession = useCallback(
    async (providerId: string, model: string) => {
      try {
        const session = await api.createSession(providerId, model);
        addSession(session);
        setMessages(session.id, []);
      } catch (e) {
        console.error("Failed to create session:", e);
      }
    },
    [addSession, setMessages]
  );

  const deleteSession = useCallback(
    async (id: string) => {
      try {
        await api.deleteSession(id);
        removeSession(id);
      } catch (e) {
        console.error("Failed to delete session:", e);
      }
    },
    [removeSession]
  );

  return { sessions, currentSessionId, selectSession, createSession, deleteSession };
}
