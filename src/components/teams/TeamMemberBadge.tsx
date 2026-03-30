import type { TeamRole } from "@/src/lib/tauri";

interface TeamMemberBadgeProps {
  name: string;
  color: string;
  role?: TeamRole | null;
}

export function TeamMemberBadge({ name, color, role }: TeamMemberBadgeProps) {
  return (
    <span
      className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs"
      style={{
        background: `${color}18`,
        color,
        border: `1px solid ${color}30`,
      }}
    >
      <span
        className="inline-block w-2 h-2 rounded-full"
        style={{ backgroundColor: color }}
      />
      {name}
      {role && (
        <span className="uppercase tracking-wide opacity-70">
          {role}
        </span>
      )}
    </span>
  );
}
