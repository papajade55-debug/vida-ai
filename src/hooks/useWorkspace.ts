import { useEffect, useCallback } from "react";
import { useStore } from "@/src/stores/store";
import { api, WorkspaceConfig, PermissionMode } from "@/src/lib/tauri";

export function useWorkspace() {
  const workspacePath = useStore((s) => s.workspacePath);
  const workspaceConfig = useStore((s) => s.workspaceConfig);
  const recentWorkspaces = useStore((s) => s.recentWorkspaces);
  const setWorkspacePath = useStore((s) => s.setWorkspacePath);
  const setWorkspaceConfig = useStore((s) => s.setWorkspaceConfig);
  const setRecentWorkspaces = useStore((s) => s.setRecentWorkspaces);

  // Load recent workspaces on mount
  useEffect(() => {
    api.listRecentWorkspaces().then(setRecentWorkspaces).catch(console.error);
  }, [setRecentWorkspaces]);

  const openWorkspace = useCallback(
    async (path: string) => {
      try {
        const config = await api.openWorkspace(path);
        setWorkspacePath(path);
        setWorkspaceConfig(config);
        // Refresh recent list
        const recent = await api.listRecentWorkspaces();
        setRecentWorkspaces(recent);
        return config;
      } catch (e) {
        console.error("Failed to open workspace:", e);
        return null;
      }
    },
    [setWorkspacePath, setWorkspaceConfig, setRecentWorkspaces]
  );

  const createWorkspace = useCallback(
    async (path: string, name: string) => {
      try {
        const config = await api.createWorkspace(path, name);
        setWorkspacePath(path);
        setWorkspaceConfig(config);
        const recent = await api.listRecentWorkspaces();
        setRecentWorkspaces(recent);
        return config;
      } catch (e) {
        console.error("Failed to create workspace:", e);
        return null;
      }
    },
    [setWorkspacePath, setWorkspaceConfig, setRecentWorkspaces]
  );

  const updateWorkspaceConfig = useCallback(
    async (config: WorkspaceConfig) => {
      try {
        await api.setWorkspaceConfig(config);
        setWorkspaceConfig(config);
      } catch (e) {
        console.error("Failed to update workspace config:", e);
      }
    },
    [setWorkspaceConfig]
  );

  const setPermissionMode = useCallback(
    async (mode: PermissionMode) => {
      try {
        await api.setPermissionMode(mode);
        if (workspaceConfig) {
          const updated = { ...workspaceConfig, permission_mode: mode };
          setWorkspaceConfig(updated);
        }
      } catch (e) {
        console.error("Failed to set permission mode:", e);
      }
    },
    [workspaceConfig, setWorkspaceConfig]
  );

  return {
    workspacePath,
    workspaceConfig,
    recentWorkspaces,
    openWorkspace,
    createWorkspace,
    updateWorkspaceConfig,
    setPermissionMode,
  };
}
