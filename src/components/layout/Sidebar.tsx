import { Plus, Settings } from "lucide-react";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import { GlassButton } from "@/src/design-system/GlassButton";
import { SessionList } from "@/src/components/sidebar/SessionList";
import { AgentList } from "@/src/components/sidebar/AgentList";
import { useStore } from "@/src/stores/store";

export function Sidebar() {
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  return (
    <GlassPanel className="h-full flex flex-col gap-2 overflow-hidden" padding="p-3">
      {/* Header */}
      <div className="flex items-center justify-between px-1">
        <span className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
          Vida AI
        </span>
        <div className="flex gap-1">
          <GlassButton variant="ghost" icon={<Plus size={16} />} title="New Chat" />
          <GlassButton
            variant="ghost"
            icon={<Settings size={16} />}
            title="Settings"
            onClick={() => setSettingsOpen(true)}
          />
        </div>
      </div>

      {/* Sessions */}
      <div className="flex-1 overflow-y-auto min-h-0">
        <div className="px-1 py-1">
          <span className="text-xs font-medium uppercase tracking-wider" style={{ color: "var(--text-secondary)" }}>
            Sessions
          </span>
        </div>
        <SessionList />
      </div>

      {/* Agents */}
      <div className="border-t pt-2" style={{ borderColor: "var(--glass-border)" }}>
        <div className="px-1 py-1">
          <span className="text-xs font-medium uppercase tracking-wider" style={{ color: "var(--text-secondary)" }}>
            Agents
          </span>
        </div>
        <AgentList />
      </div>
    </GlassPanel>
  );
}
