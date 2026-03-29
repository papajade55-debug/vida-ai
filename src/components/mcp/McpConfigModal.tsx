import { useState, useEffect } from "react";
import { X, Plus, Trash2, Save } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import type { McpServerConfigRow } from "@/src/lib/tauri";

interface McpConfigModalProps {
  open: boolean;
  onClose: () => void;
  onSave: (config: McpServerConfigRow) => Promise<void>;
  existing?: McpServerConfigRow | null;
}

interface EnvEntry {
  key: string;
  value: string;
}

export function McpConfigModal({
  open,
  onClose,
  onSave,
  existing = null,
}: McpConfigModalProps) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [envEntries, setEnvEntries] = useState<EnvEntry[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isEditing = existing !== null;

  // Initialize form when opening with existing config
  useEffect(() => {
    if (open && existing) {
      setName(existing.name);
      setCommand(existing.command);
      setArgs(
        existing.args_json
          ? JSON.parse(existing.args_json).join(" ")
          : ""
      );
      const envObj = existing.env_json
        ? JSON.parse(existing.env_json)
        : {};
      setEnvEntries(
        Object.entries(envObj).map(([key, value]) => ({
          key,
          value: String(value),
        }))
      );
      setError(null);
    } else if (open) {
      setName("");
      setCommand("");
      setArgs("");
      setEnvEntries([]);
      setError(null);
    }
  }, [open, existing]);

  const addEnvEntry = () => {
    setEnvEntries((prev) => [...prev, { key: "", value: "" }]);
  };

  const removeEnvEntry = (index: number) => {
    setEnvEntries((prev) => prev.filter((_, i) => i !== index));
  };

  const updateEnvEntry = (
    index: number,
    field: "key" | "value",
    val: string
  ) => {
    setEnvEntries((prev) =>
      prev.map((entry, i) =>
        i === index ? { ...entry, [field]: val } : entry
      )
    );
  };

  const handleSave = async () => {
    if (!name.trim() || !command.trim()) {
      setError("Name and command are required.");
      return;
    }

    setSaving(true);
    setError(null);

    try {
      const argsArray = args
        .trim()
        .split(/\s+/)
        .filter(Boolean);
      const envObj: Record<string, string> = {};
      const envKeyRegex = /^[a-zA-Z_][a-zA-Z0-9_]*$/;
      for (const entry of envEntries) {
        const key = entry.key.trim();
        if (key) {
          if (!envKeyRegex.test(key)) {
            setError(`Invalid env var key: "${key}". Only letters, digits, and underscores allowed.`);
            setSaving(false);
            return;
          }
          envObj[key] = entry.value;
        }
      }

      const config: McpServerConfigRow = {
        id: existing?.id ?? `mcp-${Date.now()}`,
        workspace_path: existing?.workspace_path ?? null,
        name: name.trim(),
        command: command.trim(),
        args_json: argsArray.length > 0 ? JSON.stringify(argsArray) : null,
        env_json:
          Object.keys(envObj).length > 0 ? JSON.stringify(envObj) : null,
        enabled: existing?.enabled ?? 1,
        created_at: existing?.created_at ?? "",
      };

      await onSave(config);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <GlassPanel className="w-full max-w-lg mx-4">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b" style={{ borderColor: "var(--glass-border)" }}>
          <h2 className="text-base font-semibold" style={{ color: "var(--text-primary)" }}>
            {isEditing ? "Edit MCP Server" : "Add MCP Server"}
          </h2>
          <GlassButton variant="ghost" icon={<X size={16} />} onClick={onClose} />
        </div>

        {/* Body */}
        <div className="p-5 space-y-4 max-h-[60vh] overflow-y-auto">
          {/* Name */}
          <div>
            <label className="block text-xs font-medium mb-1" style={{ color: "var(--text-secondary)" }}>
              Server Name
            </label>
            <GlassInput
              placeholder="e.g. filesystem"
              value={name}
              onChange={setName}
              disabled={isEditing}
            />
          </div>

          {/* Command */}
          <div>
            <label className="block text-xs font-medium mb-1" style={{ color: "var(--text-secondary)" }}>
              Command
            </label>
            <GlassInput
              placeholder="e.g. npx"
              value={command}
              onChange={setCommand}
            />
          </div>

          {/* Args */}
          <div>
            <label className="block text-xs font-medium mb-1" style={{ color: "var(--text-secondary)" }}>
              Arguments (space-separated)
            </label>
            <GlassInput
              placeholder="e.g. -y @modelcontextprotocol/server-filesystem /tmp"
              value={args}
              onChange={setArgs}
            />
          </div>

          {/* Environment Variables */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
                Environment Variables
              </label>
              <GlassButton
                variant="ghost"
                icon={<Plus size={12} />}
                onClick={addEnvEntry}
                className="!px-2 !py-0.5 text-xs"
              >
                Add
              </GlassButton>
            </div>
            {envEntries.length === 0 && (
              <p className="text-xs" style={{ color: "var(--text-secondary)" }}>
                No environment variables. Click "Add" to define one.
              </p>
            )}
            <div className="space-y-2">
              {envEntries.map((entry, i) => (
                <div key={i} className="flex items-center gap-2">
                  <GlassInput
                    placeholder="KEY"
                    value={entry.key}
                    onChange={(v) => updateEnvEntry(i, "key", v)}
                  />
                  <span className="text-xs" style={{ color: "var(--text-secondary)" }}>=</span>
                  <GlassInput
                    placeholder="value"
                    value={entry.value}
                    onChange={(v) => updateEnvEntry(i, "value", v)}
                  />
                  <GlassButton
                    variant="ghost"
                    icon={<Trash2 size={12} />}
                    onClick={() => removeEnvEntry(i)}
                    className="!px-1.5 !py-1 flex-shrink-0"
                  />
                </div>
              ))}
            </div>
          </div>

          {/* Error */}
          {error && (
            <div className="text-xs px-3 py-2 rounded" style={{ color: "#ef4444", background: "#ef444420" }}>
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-4 border-t" style={{ borderColor: "var(--glass-border)" }}>
          <GlassButton variant="ghost" onClick={onClose} disabled={saving}>
            Cancel
          </GlassButton>
          <GlassButton
            variant="primary"
            icon={<Save size={14} />}
            onClick={handleSave}
            disabled={saving || !name.trim() || !command.trim()}
          >
            {saving ? "Saving…" : isEditing ? "Update" : "Add Server"}
          </GlassButton>
        </div>
      </GlassPanel>
    </div>
  );
}
