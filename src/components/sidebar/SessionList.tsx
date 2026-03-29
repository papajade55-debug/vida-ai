import { useSessions } from "@/src/hooks/useSessions";
import { SessionItem } from "./SessionItem";

export function SessionList() {
  const { sessions, currentSessionId, selectSession, deleteSession } = useSessions();

  if (sessions.length === 0) {
    return (
      <div className="px-3 py-4 text-center text-xs" style={{ color: "var(--text-secondary)" }}>
        No sessions yet
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      {sessions.map((session) => (
        <SessionItem
          key={session.id}
          session={session}
          active={session.id === currentSessionId}
          onSelect={() => selectSession(session.id)}
          onDelete={() => deleteSession(session.id)}
        />
      ))}
    </div>
  );
}
