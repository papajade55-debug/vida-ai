import { useEffect, useCallback } from "react";
import { useStore } from "@/src/stores/store";
import { api } from "@/src/lib/tauri";
import type { McpServerConfigRow } from "@/src/lib/tauri";

export function useMcp() {
  const mcpServers = useStore((s) => s.mcpServers);
  const setMcpServers = useStore((s) => s.setMcpServers);

  useEffect(() => {
    api.listMcpServers().then(setMcpServers).catch(console.error);
  }, [setMcpServers]);

  const refreshServers = useCallback(async () => {
    try {
      const servers = await api.listMcpServers();
      setMcpServers(servers);
    } catch (e) {
      console.error("Failed to refresh MCP servers:", e);
    }
  }, [setMcpServers]);

  const startServer = useCallback(
    async (name: string) => {
      try {
        await api.startMcpServer(name);
        await refreshServers();
      } catch (e) {
        console.error("Failed to start MCP server:", e);
        throw e;
      }
    },
    [refreshServers]
  );

  const stopServer = useCallback(
    async (name: string) => {
      try {
        await api.stopMcpServer(name);
        await refreshServers();
      } catch (e) {
        console.error("Failed to stop MCP server:", e);
        throw e;
      }
    },
    [refreshServers]
  );

  const saveConfig = useCallback(
    async (config: McpServerConfigRow) => {
      try {
        await api.saveMcpServerConfig(config);
        await refreshServers();
      } catch (e) {
        console.error("Failed to save MCP server config:", e);
        throw e;
      }
    },
    [refreshServers]
  );

  const deleteConfig = useCallback(
    async (id: string) => {
      try {
        await api.deleteMcpServerConfig(id);
        await refreshServers();
      } catch (e) {
        console.error("Failed to delete MCP server config:", e);
        throw e;
      }
    },
    [refreshServers]
  );

  return {
    mcpServers,
    startServer,
    stopServer,
    saveConfig,
    deleteConfig,
    refreshServers,
  };
}
