import { useEffect, useCallback } from "react";
import { useStore } from "@/src/stores/store";
import { api } from "@/src/lib/tauri";

export function useTeams() {
  const teams = useStore((s) => s.teams);
  const setTeams = useStore((s) => s.setTeams);
  const addTeam = useStore((s) => s.addTeam);
  const removeTeam = useStore((s) => s.removeTeam);

  useEffect(() => {
    api.listTeams().then(setTeams).catch(console.error);
  }, [setTeams]);

  const createTeam = useCallback(
    async (name: string, members: [string, string][]) => {
      try {
        const team = await api.createTeam(name, members);
        addTeam(team);
        return team;
      } catch (e) {
        console.error("Failed to create team:", e);
        return null;
      }
    },
    [addTeam]
  );

  const deleteTeam = useCallback(
    async (id: string) => {
      try {
        await api.deleteTeam(id);
        removeTeam(id);
      } catch (e) {
        console.error("Failed to delete team:", e);
      }
    },
    [removeTeam]
  );

  return { teams, createTeam, deleteTeam };
}
