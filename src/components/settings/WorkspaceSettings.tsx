import { useWorkspace } from "@/src/hooks/useWorkspace";
import type { PermissionMode } from "@/src/lib/tauri";

const PERMISSION_MODES: { value: PermissionMode; label: string; description: string }[] = [
  { value: "yolo", label: "Yolo", description: "All actions allowed without confirmation" },
  { value: "ask", label: "Ask", description: "Confirm each action (default)" },
  { value: "sandbox", label: "Sandbox", description: "All actions denied unless explicitly granted" },
];

export function WorkspaceSettings() {
  const { workspaceConfig, workspacePath, updateWorkspaceConfig, setPermissionMode } = useWorkspace();

  if (!workspaceConfig || !workspacePath) {
    return (
      <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
        No workspace open. Open or create a workspace from the sidebar to configure it.
      </div>
    );
  }

  const handlePermissionToggle = (key: keyof typeof workspaceConfig.permissions) => {
    const updated = {
      ...workspaceConfig,
      permissions: {
        ...workspaceConfig.permissions,
        [key]: !workspaceConfig.permissions[key],
      },
    };
    updateWorkspaceConfig(updated);
  };

  return (
    <div className="space-y-6">
      {/* Workspace info */}
      <div>
        <label
          className="block text-sm font-medium mb-1"
          style={{ color: "var(--text-primary)" }}
        >
          Workspace
        </label>
        <div
          className="text-xs font-mono px-3 py-2 rounded-[var(--radius)]"
          style={{
            background: "var(--glass-bg)",
            color: "var(--text-secondary)",
            border: "1px solid var(--glass-border)",
          }}
        >
          {workspacePath}
        </div>
      </div>

      {/* Permission Mode */}
      <div>
        <label
          className="block text-sm font-medium mb-2"
          style={{ color: "var(--text-primary)" }}
        >
          Permission Mode
        </label>
        <div className="space-y-2">
          {PERMISSION_MODES.map((mode) => (
            <button
              key={mode.value}
              onClick={() => setPermissionMode(mode.value)}
              className="flex flex-col w-full px-3 py-2 rounded-[var(--radius)] text-left cursor-pointer transition-all"
              style={{
                background:
                  workspaceConfig.permission_mode === mode.value
                    ? "var(--accent)"
                    : "var(--glass-bg)",
                color:
                  workspaceConfig.permission_mode === mode.value
                    ? "var(--accent-text)"
                    : "var(--text-primary)",
                border: `1px solid ${
                  workspaceConfig.permission_mode === mode.value
                    ? "transparent"
                    : "var(--glass-border)"
                }`,
              }}
            >
              <span className="text-sm font-medium">{mode.label}</span>
              <span
                className="text-xs"
                style={{
                  opacity: workspaceConfig.permission_mode === mode.value ? 0.9 : 0.6,
                }}
              >
                {mode.description}
              </span>
            </button>
          ))}
        </div>
      </div>

      {/* Permission Toggles */}
      <div>
        <label
          className="block text-sm font-medium mb-2"
          style={{ color: "var(--text-primary)" }}
        >
          Permissions
        </label>
        <div className="space-y-1">
          {(
            [
              { key: "file_read" as const, label: "File Read", desc: "Read files from disk" },
              { key: "file_write" as const, label: "File Write", desc: "Write/create/delete files" },
              { key: "shell_execute" as const, label: "Shell Execute", desc: "Execute shell commands" },
              { key: "network_access" as const, label: "Network Access", desc: "Make network requests" },
            ] as const
          ).map((perm) => (
            <label
              key={perm.key}
              className="flex items-center justify-between px-3 py-2 rounded-[var(--radius)] cursor-pointer"
              style={{
                background: "var(--glass-bg)",
                border: "1px solid var(--glass-border)",
              }}
            >
              <div>
                <div className="text-sm" style={{ color: "var(--text-primary)" }}>
                  {perm.label}
                </div>
                <div className="text-xs" style={{ color: "var(--text-secondary)" }}>
                  {perm.desc}
                </div>
              </div>
              <input
                type="checkbox"
                checked={workspaceConfig.permissions[perm.key]}
                onChange={() => handlePermissionToggle(perm.key)}
                className="accent-[var(--accent)] w-4 h-4"
              />
            </label>
          ))}
        </div>
      </div>
    </div>
  );
}
