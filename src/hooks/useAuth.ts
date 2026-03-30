import { useCallback, useEffect, useState } from "react";
import { api, type ActorRole } from "@/src/lib/tauri";
import { useStore } from "@/src/stores/store";

export function useAuth() {
  const authActor = useStore((s) => s.authActor);
  const authUsers = useStore((s) => s.authUsers);
  const setAuthActor = useStore((s) => s.setAuthActor);
  const setAuthUsers = useStore((s) => s.setAuthUsers);
  const [hasUsers, setHasUsers] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    const status = await api.getAuthStatus();
    setHasUsers(status.has_users);
    setAuthActor(status.actor);
    if (status.actor) {
      try {
        setAuthUsers(await api.listUsers());
      } catch {
        setAuthUsers([]);
      }
    } else {
      setAuthUsers([]);
    }
    return status;
  }, [setAuthActor, setAuthUsers]);

  useEffect(() => {
    refresh()
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [refresh]);

  const bootstrap = useCallback(
    async (username: string, password: string) => {
      const actor = await api.bootstrapLocalAdmin(username, password);
      setAuthActor(actor);
      setHasUsers(true);
      setAuthUsers(await api.listUsers());
      return actor;
    },
    [setAuthActor, setAuthUsers],
  );

  const login = useCallback(
    async (username: string, password: string) => {
      const actor = await api.loginLocal(username, password);
      setAuthActor(actor);
      setHasUsers(true);
      setAuthUsers(await api.listUsers());
      return actor;
    },
    [setAuthActor, setAuthUsers],
  );

  const logout = useCallback(async () => {
    await api.logoutLocal();
    setAuthActor(null);
    setAuthUsers([]);
  }, [setAuthActor, setAuthUsers]);

  const createUser = useCallback(
    async (username: string, password: string, role: ActorRole) => {
      const user = await api.createUser(username, password, role);
      setAuthUsers(await api.listUsers());
      return user;
    },
    [setAuthUsers],
  );

  const changePassword = useCallback(async (currentPassword: string, newPassword: string) => {
    await api.changePassword(currentPassword, newPassword);
  }, []);

  return {
    authActor,
    authUsers,
    hasUsers,
    loading,
    refresh,
    bootstrap,
    login,
    logout,
    createUser,
    changePassword,
  };
}
