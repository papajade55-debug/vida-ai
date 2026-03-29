import { useEffect, useCallback } from "react";
import { useStore } from "@/src/stores/store";
import { api } from "@/src/lib/tauri";

export function useProviders() {
  const providers = useStore((s) => s.providers);
  const health = useStore((s) => s.providerHealth);
  const setProviders = useStore((s) => s.setProviders);
  const setProviderHealth = useStore((s) => s.setProviderHealth);

  const refresh = useCallback(async () => {
    try {
      const providerList = await api.listProviders();
      setProviders(providerList);
      const healthResults = await api.healthCheck();
      const healthMap: Record<string, boolean> = {};
      for (const [name, ok] of healthResults) {
        healthMap[name] = ok;
      }
      setProviderHealth(healthMap);
    } catch (e) {
      console.error("Failed to load providers:", e);
    }
  }, [setProviders, setProviderHealth]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 60_000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { providers, health, refresh };
}
