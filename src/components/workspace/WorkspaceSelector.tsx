import { useState, useRef, useEffect } from "react";
import { FolderOpen, ChevronDown, Clock, Plus } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useWorkspace } from "@/src/hooks/useWorkspace";

export function WorkspaceSelector() {
  const {
    workspacePath,
    workspaceConfig,
    recentWorkspaces,
    openWorkspace,
    createWorkspace,
  } = useWorkspace();

  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [showInput, setShowInput] = useState(false);
  const [inputPath, setInputPath] = useState("");
  const [inputName, setInputName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
        setShowInput(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const handleOpenRecent = async (path: string) => {
    await openWorkspace(path);
    setDropdownOpen(false);
  };

  const handleCreate = async () => {
    if (!inputPath.trim()) return;
    setIsCreating(true);
    if (isCreating) {
      await createWorkspace(inputPath.trim(), inputName.trim() || "Untitled");
    } else {
      await openWorkspace(inputPath.trim());
    }
    setDropdownOpen(false);
    setShowInput(false);
    setInputPath("");
    setInputName("");
    setIsCreating(false);
  };

  const displayName = workspaceConfig?.name || "No Workspace";

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => setDropdownOpen(!dropdownOpen)}
        className="flex items-center gap-2 w-full px-2 py-1.5 rounded-[var(--radius)] text-xs font-medium cursor-pointer transition-colors hover:opacity-80"
        style={{
          background: "var(--glass-bg)",
          color: "var(--text-primary)",
          border: "1px solid var(--glass-border)",
        }}
      >
        <FolderOpen size={14} style={{ color: "var(--accent)" }} />
        <span className="flex-1 text-left truncate">{displayName}</span>
        <ChevronDown size={12} style={{ color: "var(--text-secondary)" }} />
      </button>

      {dropdownOpen && (
        <div
          className="absolute left-0 right-0 top-full mt-1 z-50 rounded-[var(--radius)] overflow-hidden"
          style={{
            background: "var(--bg-secondary)",
            border: "1px solid var(--glass-border)",
            boxShadow: "var(--glass-shadow)",
          }}
        >
          {/* Recent workspaces */}
          {recentWorkspaces.length > 0 && (
            <div className="py-1">
              <div
                className="px-3 py-1 text-[10px] uppercase tracking-wider font-medium"
                style={{ color: "var(--text-secondary)" }}
              >
                Recent
              </div>
              {recentWorkspaces.map((ws) => (
                <button
                  key={ws.path}
                  onClick={() => handleOpenRecent(ws.path)}
                  className="flex items-center gap-2 w-full px-3 py-1.5 text-xs cursor-pointer transition-colors hover:opacity-80"
                  style={{ color: "var(--text-primary)" }}
                  title={ws.path}
                >
                  <Clock size={12} style={{ color: "var(--text-secondary)" }} />
                  <span className="truncate">{ws.name}</span>
                </button>
              ))}
            </div>
          )}

          {/* Divider */}
          {recentWorkspaces.length > 0 && (
            <div style={{ borderTop: "1px solid var(--glass-border)" }} />
          )}

          {/* Open / Create */}
          {!showInput ? (
            <div className="py-1">
              <button
                onClick={() => {
                  setShowInput(true);
                  setIsCreating(false);
                }}
                className="flex items-center gap-2 w-full px-3 py-1.5 text-xs cursor-pointer transition-colors hover:opacity-80"
                style={{ color: "var(--text-primary)" }}
              >
                <FolderOpen size={12} style={{ color: "var(--accent)" }} />
                Open Folder...
              </button>
              <button
                onClick={() => {
                  setShowInput(true);
                  setIsCreating(true);
                }}
                className="flex items-center gap-2 w-full px-3 py-1.5 text-xs cursor-pointer transition-colors hover:opacity-80"
                style={{ color: "var(--text-primary)" }}
              >
                <Plus size={12} style={{ color: "var(--accent)" }} />
                New Workspace...
              </button>
            </div>
          ) : (
            <div className="p-2 space-y-2">
              <input
                type="text"
                placeholder="Folder path..."
                value={inputPath}
                onChange={(e) => setInputPath(e.target.value)}
                className="w-full px-2 py-1 text-xs rounded-[var(--radius)] outline-none"
                style={{
                  background: "var(--glass-bg)",
                  color: "var(--text-primary)",
                  border: "1px solid var(--glass-border)",
                }}
                autoFocus
                onKeyDown={(e) => e.key === "Enter" && handleCreate()}
              />
              {isCreating && (
                <input
                  type="text"
                  placeholder="Workspace name..."
                  value={inputName}
                  onChange={(e) => setInputName(e.target.value)}
                  className="w-full px-2 py-1 text-xs rounded-[var(--radius)] outline-none"
                  style={{
                    background: "var(--glass-bg)",
                    color: "var(--text-primary)",
                    border: "1px solid var(--glass-border)",
                  }}
                  onKeyDown={(e) => e.key === "Enter" && handleCreate()}
                />
              )}
              <GlassButton
                variant="primary"
                onClick={handleCreate}
                className="w-full !text-xs !py-1"
              >
                {isCreating ? "Create" : "Open"}
              </GlassButton>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
