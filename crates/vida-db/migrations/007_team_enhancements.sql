-- Add system_prompt to teams (for team-wide instructions)
ALTER TABLE teams ADD COLUMN system_prompt TEXT;
ALTER TABLE teams ADD COLUMN description TEXT;

-- Add system_prompt and department to team_members (per-agent personality)
ALTER TABLE team_members ADD COLUMN system_prompt TEXT;
ALTER TABLE team_members ADD COLUMN department TEXT;
