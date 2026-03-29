import { Trash2 } from "lucide-react";
import { GlassCard } from "@/src/design-system/GlassCard";
import type { SessionRow } from "@/src/lib/tauri";

interface SessionItemProps {
  session: SessionRow;
  active: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

export function SessionItem({ session, active, onSelect, onDelete }: SessionItemProps) {
  return (
    <GlassCard active={active} onClick={onSelect} className="group flex items-center justify-between">
      <div className="min-w-0 flex-1">
        <div className="text-sm truncate" style={{ color: "var(--text-primary)" }}>
          {session.title || `Chat ${session.model}`}
        </div>
        <div className="text-xs truncate" style={{ color: "var(--text-secondary)" }}>
          {session.model}
        </div>
      </div>
      <button
        onClick={(e) => { e.stopPropagation(); onDelete(); }}
        className="opacity-0 group-hover:opacity-60 hover:opacity-100 transition-opacity p-1"
        style={{ color: "var(--text-secondary)" }}
      >
        <Trash2 size={14} />
      </button>
    </GlassCard>
  );
}
