import { Trash2 } from "lucide-react";
import { GlassCard } from "@/src/design-system/GlassCard";
import { TeamMemberBadge } from "./TeamMemberBadge";
import type { TeamRow, TeamMemberRow } from "@/src/lib/tauri";

interface TeamItemProps {
  team: TeamRow;
  members: TeamMemberRow[];
  active?: boolean;
  onClick?: () => void;
  onDelete?: () => void;
}

export function TeamItem({ team, members, active, onClick, onDelete }: TeamItemProps) {
  return (
    <GlassCard active={active} onClick={onClick} className="group">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium truncate" style={{ color: "var(--text-primary)" }}>
          {team.name}
        </span>
        {onDelete && (
          <button
            onClick={(e) => { e.stopPropagation(); onDelete(); }}
            className="opacity-0 group-hover:opacity-60 hover:opacity-100 transition-opacity p-1"
            style={{ color: "var(--text-secondary)" }}
          >
            <Trash2 size={14} />
          </button>
        )}
      </div>
      {members.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-1.5">
          {members.map((m) => (
            <TeamMemberBadge
              key={m.id}
              name={m.display_name || m.model}
              color={m.color}
            />
          ))}
        </div>
      )}
    </GlassCard>
  );
}
