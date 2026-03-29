import { useState, useEffect } from "react";
import { useTeams } from "@/src/hooks/useTeams";
import { TeamItem } from "./TeamItem";
import { api } from "@/src/lib/tauri";
import type { TeamMemberRow } from "@/src/lib/tauri";

export function TeamList() {
  const { teams, deleteTeam } = useTeams();
  const [teamMembers, setTeamMembers] = useState<Record<string, TeamMemberRow[]>>({});

  // Load members for each team
  useEffect(() => {
    for (const team of teams) {
      if (!teamMembers[team.id]) {
        api.getTeam(team.id).then(([, members]) => {
          setTeamMembers((prev) => ({ ...prev, [team.id]: members }));
        }).catch(console.error);
      }
    }
  }, [teams, teamMembers]);

  if (teams.length === 0) {
    return (
      <div className="px-3 py-2 text-center text-xs" style={{ color: "var(--text-secondary)" }}>
        No teams yet
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      {teams.map((team) => (
        <TeamItem
          key={team.id}
          team={team}
          members={teamMembers[team.id] ?? []}
          onDelete={() => deleteTeam(team.id)}
        />
      ))}
    </div>
  );
}
