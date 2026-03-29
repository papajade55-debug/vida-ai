import { Menu } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useStore } from "@/src/stores/store";

export function ChatHeader() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const sessions = useStore((s) => s.sessions);
  const toggleSidebar = useStore((s) => s.toggleSidebar);

  const session = sessions.find((s) => s.id === currentSessionId);

  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b" style={{ borderColor: "var(--glass-border)" }}>
      <GlassButton variant="ghost" icon={<Menu size={18} />} onClick={toggleSidebar} />
      {session ? (
        <div>
          <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
            {session.title || `Chat with ${session.model}`}
          </div>
          <div className="text-xs" style={{ color: "var(--text-secondary)" }}>
            {session.provider_id} · {session.model}
          </div>
        </div>
      ) : (
        <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
          Select or create a session
        </div>
      )}
    </div>
  );
}
