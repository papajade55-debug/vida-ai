import { useState, useMemo } from "react";
import { X } from "lucide-react";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { useProviders } from "@/src/hooks/useProviders";
import { useTeams } from "@/src/hooks/useTeams";
import { TeamMemberBadge } from "./TeamMemberBadge";

const TEAM_COLORS = [
  "#6366f1", "#ec4899", "#14b8a6", "#f59e0b",
  "#8b5cf6", "#06b6d4", "#f97316", "#10b981",
];

interface TeamCreatorProps {
  open: boolean;
  onClose: () => void;
}

export function TeamCreator({ open, onClose }: TeamCreatorProps) {
  const { providers } = useProviders();
  const { createTeam } = useTeams();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());

  // Build flat list of provider/model pairs
  const allModels = useMemo(() => {
    const models: { key: string; providerId: string; model: string; label: string }[] = [];
    for (const p of providers) {
      for (const m of p.models) {
        models.push({
          key: `${p.id}/${m}`,
          providerId: p.id,
          model: m,
          label: `${p.display_name}/${m}`,
        });
      }
    }
    return models;
  }, [providers]);

  const toggleModel = (key: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const selectedEntries = useMemo(() => {
    return allModels.filter((m) => selected.has(m.key));
  }, [allModels, selected]);

  const handleCreate = async () => {
    if (!name.trim() || selectedEntries.length === 0) return;
    const members: [string, string][] = selectedEntries.map((e) => [e.providerId, e.model]);
    await createTeam(
      name.trim(),
      members,
      description.trim() || undefined,
      systemPrompt.trim() || undefined,
    );
    setName("");
    setDescription("");
    setSystemPrompt("");
    setSelected(new Set());
    onClose();
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0"
        style={{ background: "rgba(0,0,0,0.5)", backdropFilter: "blur(4px)" }}
        onClick={onClose}
      />
      {/* Modal */}
      <GlassPanel className="relative z-10 w-full max-w-lg max-h-[80vh] flex flex-col" padding="p-5">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
            Create Team
          </h2>
          <GlassButton variant="ghost" icon={<X size={18} />} onClick={onClose} />
        </div>

        {/* Team name */}
        <div className="mb-4">
          <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
            Team Name
          </label>
          <GlassInput
            value={name}
            onChange={setName}
            placeholder="e.g. Code Review Team"
          />
        </div>

        {/* Description */}
        <div className="mb-4">
          <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
            Description (optional)
          </label>
          <GlassInput
            value={description}
            onChange={setDescription}
            placeholder="e.g. Team for code review and architecture"
          />
        </div>

        {/* System Prompt */}
        <div className="mb-4">
          <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
            System Prompt (optional)
          </label>
          <textarea
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            placeholder="Instructions shared by all team members..."
            rows={3}
            className="w-full px-3 py-2 rounded-[var(--radius)] text-sm resize-none"
            style={{
              background: "var(--glass-bg)",
              border: "1px solid var(--glass-border)",
              color: "var(--text-primary)",
              outline: "none",
            }}
          />
        </div>

        {/* Model selection grid */}
        <div className="mb-4 flex-1 overflow-y-auto">
          <label className="text-xs font-medium mb-2 block" style={{ color: "var(--text-secondary)" }}>
            Select Models
          </label>
          <div className="grid grid-cols-1 gap-1">
            {allModels.map((m) => {
              const isChecked = selected.has(m.key);
              return (
                <label
                  key={m.key}
                  className="flex items-center gap-2 px-3 py-2 rounded-[var(--radius)] cursor-pointer transition-colors"
                  style={{
                    background: isChecked ? "rgba(99, 102, 241, 0.1)" : "transparent",
                    border: isChecked ? "1px solid var(--accent)" : "1px solid transparent",
                  }}
                >
                  <input
                    type="checkbox"
                    checked={isChecked}
                    onChange={() => toggleModel(m.key)}
                    className="accent-[var(--accent)]"
                  />
                  <span className="text-sm" style={{ color: "var(--text-primary)" }}>
                    {m.label}
                  </span>
                </label>
              );
            })}
          </div>
        </div>

        {/* Selected preview */}
        {selectedEntries.length > 0 && (
          <div className="mb-4">
            <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
              Team Members ({selectedEntries.length})
            </label>
            <div className="flex flex-wrap gap-1">
              {selectedEntries.map((entry, i) => (
                <TeamMemberBadge
                  key={entry.key}
                  name={entry.label}
                  color={TEAM_COLORS[i % TEAM_COLORS.length]}
                  role={i === 0 ? "owner" : "member"}
                />
              ))}
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <GlassButton variant="secondary" onClick={onClose}>
            Cancel
          </GlassButton>
          <GlassButton
            variant="primary"
            onClick={handleCreate}
            disabled={!name.trim() || selectedEntries.length === 0}
          >
            Create Team
          </GlassButton>
        </div>
      </GlassPanel>
    </div>
  );
}
