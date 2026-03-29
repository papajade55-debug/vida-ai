import { useState } from "react";
import { Plus, RefreshCw, Plug } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { useMcp } from "@/src/hooks/useMcp";
import { McpServerCard } from "./McpServerCard";
import type { McpServerConfigRow } from "@/src/lib/tauri";

export function McpPanel() {
  const { mcpServers, startServer, stopServer, saveConfig, deleteConfig, refreshServers } =
    useMcp();
  const [loading, setLoading] = useState(false);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState("");
  const [newCommand, setNewCommand] = useState("");
  const [newArgs, setNewArgs] = useState("");

  const handleStart = async (name: string) => {
    setLoading(true);
    try {
      await startServer(name);
    } catch {
      // error already logged in hook
    }
    setLoading(false);
  };

  const handleStop = async (name: string) => {
    setLoading(true);
    try {
      await stopServer(name);
    } catch {
      // error already logged in hook
    }
    setLoading(false);
  };

  const handleDelete = async (name: string) => {
    const server = mcpServers.find((s) => s.name === name);
    if (!server) return;
    setLoading(true);
    try {
      // Find the config ID - use the name as lookup
      // We'll delete by finding the server then calling deleteConfig
      await deleteConfig(name);
    } catch {
      // error already logged in hook
    }
    setLoading(false);
  };

  const handleAdd = async () => {
    if (!newName.trim() || !newCommand.trim()) return;
    setLoading(true);
    try {
      const id = `mcp-${Date.now()}`;
      const argsJson = newArgs.trim()
        ? JSON.stringify(newArgs.split(" ").filter(Boolean))
        : null;
      const config: McpServerConfigRow = {
        id,
        workspace_path: null,
        name: newName.trim(),
        command: newCommand.trim(),
        args_json: argsJson,
        env_json: null,
        enabled: 1,
        created_at: "",
      };
      await saveConfig(config);
      setNewName("");
      setNewCommand("");
      setNewArgs("");
      setShowAdd(false);
    } catch {
      // error already logged in hook
    }
    setLoading(false);
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Plug size={16} style={{ color: "var(--accent)" }} />
          <h3
            className="text-sm font-semibold"
            style={{ color: "var(--text-primary)" }}
          >
            MCP Servers
          </h3>
          <span
            className="text-xs"
            style={{ color: "var(--text-secondary)" }}
          >
            ({mcpServers.length})
          </span>
        </div>
        <div className="flex gap-1">
          <GlassButton
            variant="ghost"
            icon={<RefreshCw size={14} />}
            onClick={refreshServers}
            className="!px-2 !py-1"
            title="Refresh"
          />
          <GlassButton
            variant="ghost"
            icon={<Plus size={14} />}
            onClick={() => setShowAdd(!showAdd)}
            className="!px-2 !py-1"
            title="Add server"
          />
        </div>
      </div>

      {/* Add form */}
      {showAdd && (
        <div
          className="space-y-2 p-3 rounded-[var(--radius)]"
          style={{
            background: "var(--glass-bg)",
            border: "1px solid var(--glass-border)",
          }}
        >
          <GlassInput
            placeholder="Server name (e.g. filesystem)"
            value={newName}
            onChange={(v) => setNewName(v)}
          />
          <GlassInput
            placeholder="Command (e.g. npx)"
            value={newCommand}
            onChange={(v) => setNewCommand(v)}
          />
          <GlassInput
            placeholder="Args (space-separated, e.g. -y @modelcontextprotocol/server-filesystem /tmp)"
            value={newArgs}
            onChange={(v) => setNewArgs(v)}
          />
          <div className="flex gap-2 justify-end">
            <GlassButton
              variant="ghost"
              onClick={() => setShowAdd(false)}
            >
              Cancel
            </GlassButton>
            <GlassButton
              variant="primary"
              onClick={handleAdd}
              disabled={loading || !newName.trim() || !newCommand.trim()}
            >
              Add
            </GlassButton>
          </div>
        </div>
      )}

      {/* Server list */}
      <div className="space-y-2">
        {mcpServers.length === 0 ? (
          <div
            className="text-center py-6 text-sm"
            style={{ color: "var(--text-secondary)" }}
          >
            No MCP servers configured. Click + to add one.
          </div>
        ) : (
          mcpServers.map((server) => (
            <McpServerCard
              key={server.name}
              server={server}
              onStart={handleStart}
              onStop={handleStop}
              onDelete={handleDelete}
              loading={loading}
            />
          ))
        )}
      </div>
    </div>
  );
}
