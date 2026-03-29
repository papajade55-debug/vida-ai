import { Play, Square, Trash2, Wrench } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { StatusDot } from "@/src/design-system/StatusDot";
import type { McpServerInfo } from "@/src/lib/tauri";

interface McpServerCardProps {
  server: McpServerInfo;
  onStart: (name: string) => void;
  onStop: (name: string) => void;
  onDelete: (name: string) => void;
  loading?: boolean;
}

export function McpServerCard({
  server,
  onStart,
  onStop,
  onDelete,
  loading = false,
}: McpServerCardProps) {
  return (
    <div
      className="flex items-center justify-between px-3 py-3 rounded-[var(--radius)]"
      style={{
        background: "var(--glass-bg)",
        border: "1px solid var(--glass-border)",
      }}
    >
      <div className="flex items-center gap-3 min-w-0">
        <StatusDot status={server.running ? "idle" : "offline"} />
        <div className="min-w-0">
          <div
            className="text-sm font-medium truncate"
            style={{ color: "var(--text-primary)" }}
          >
            {server.name}
          </div>
          <div
            className="text-xs truncate"
            style={{ color: "var(--text-secondary)" }}
          >
            {server.command}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2 flex-shrink-0">
        {server.running && (
          <div
            className="flex items-center gap-1 text-xs px-2 py-0.5 rounded-full"
            style={{
              background: "var(--accent-10)",
              color: "var(--accent)",
            }}
          >
            <Wrench size={10} />
            {server.tool_count}
          </div>
        )}

        {server.running ? (
          <GlassButton
            variant="ghost"
            icon={<Square size={14} />}
            onClick={() => onStop(server.name)}
            disabled={loading}
            className="!px-2 !py-1"
            title="Stop server"
          />
        ) : (
          <GlassButton
            variant="ghost"
            icon={<Play size={14} />}
            onClick={() => onStart(server.name)}
            disabled={loading}
            className="!px-2 !py-1"
            title="Start server"
          />
        )}

        <GlassButton
          variant="ghost"
          icon={<Trash2 size={14} />}
          onClick={() => onDelete(server.name)}
          disabled={loading}
          className="!px-2 !py-1"
          title="Remove server"
        />
      </div>
    </div>
  );
}
