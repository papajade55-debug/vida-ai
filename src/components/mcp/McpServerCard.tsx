import { useState } from "react";
import { Play, Square, Trash2, Wrench, Settings, ChevronDown, ChevronUp } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { StatusDot } from "@/src/design-system/StatusDot";
import type { McpServerInfo } from "@/src/lib/tauri";

interface McpServerCardProps {
  server: McpServerInfo;
  onStart: (name: string) => void;
  onStop: (name: string) => void;
  onDelete: (name: string) => void;
  onEdit: (server: McpServerInfo) => void;
  loading?: boolean;
}

export function McpServerCard({
  server,
  onStart,
  onStop,
  onDelete,
  onEdit,
  loading = false,
}: McpServerCardProps) {
  const [showTools, setShowTools] = useState(false);

  return (
    <div
      className="rounded-[var(--radius)]"
      style={{
        background: "var(--glass-bg)",
        border: "1px solid var(--glass-border)",
      }}
    >
      {/* Main row */}
      <div className="flex items-center justify-between px-3 py-3">
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
          {server.running && server.tool_count > 0 && (
            <button
              onClick={() => setShowTools(!showTools)}
              className="flex items-center gap-1 text-xs px-2 py-0.5 rounded-full cursor-pointer hover:opacity-80 transition-opacity"
              style={{
                background: "var(--accent-10)",
                color: "var(--accent)",
                border: "none",
              }}
            >
              <Wrench size={10} />
              {server.tool_count}
              {showTools ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
            </button>
          )}

          <GlassButton
            variant="ghost"
            icon={<Settings size={14} />}
            onClick={() => onEdit(server)}
            disabled={loading}
            className="!px-2 !py-1"
            title="Edit server config"
          />

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

      {/* Expandable tools list */}
      {showTools && server.tools.length > 0 && (
        <div
          className="px-3 pb-3 border-t"
          style={{ borderColor: "var(--glass-border)" }}
        >
          <div className="pt-2 space-y-1">
            <div
              className="text-xs font-medium mb-1"
              style={{ color: "var(--text-secondary)" }}
            >
              Available Tools
            </div>
            {server.tools.map((tool) => (
              <div
                key={tool.name}
                className="flex items-start gap-2 px-2 py-1.5 rounded text-xs"
                style={{ background: "var(--glass-bg)" }}
              >
                <Wrench
                  size={10}
                  className="mt-0.5 flex-shrink-0"
                  style={{ color: "var(--accent)" }}
                />
                <div className="min-w-0">
                  <span
                    className="font-mono font-medium"
                    style={{ color: "var(--text-primary)" }}
                  >
                    {tool.name}
                  </span>
                  {tool.description && (
                    <p
                      className="mt-0.5 truncate"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      {tool.description}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
