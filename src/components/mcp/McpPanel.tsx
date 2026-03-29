import { useState } from "react";
import { Plus, RefreshCw, Plug } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useMcp } from "@/src/hooks/useMcp";
import { McpServerCard } from "./McpServerCard";
import { McpConfigModal } from "./McpConfigModal";
import type { McpServerConfigRow, McpServerInfo } from "@/src/lib/tauri";

export function McpPanel() {
  const { mcpServers, startServer, stopServer, saveConfig, deleteConfig, refreshServers } =
    useMcp();
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingServer, setEditingServer] = useState<McpServerConfigRow | null>(null);

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
    setLoading(true);
    try {
      await deleteConfig(name);
    } catch {
      // error already logged in hook
    }
    setLoading(false);
  };

  const handleEdit = (server: McpServerInfo) => {
    // Build a McpServerConfigRow from the McpServerInfo for editing
    const config: McpServerConfigRow = {
      id: `mcp-${server.name}`,
      workspace_path: null,
      name: server.name,
      command: server.command,
      args_json: null,
      env_json: null,
      enabled: 1,
      created_at: "",
    };
    setEditingServer(config);
    setModalOpen(true);
  };

  const handleSave = async (config: McpServerConfigRow) => {
    await saveConfig(config);
  };

  const openAddModal = () => {
    setEditingServer(null);
    setModalOpen(true);
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
            onClick={openAddModal}
            className="!px-2 !py-1"
            title="Add server"
          />
        </div>
      </div>

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
              onEdit={handleEdit}
              loading={loading}
            />
          ))
        )}
      </div>

      {/* Add/Edit Modal */}
      <McpConfigModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onSave={handleSave}
        existing={editingServer}
      />
    </div>
  );
}
