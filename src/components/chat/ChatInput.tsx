import { useState, useEffect } from "react";
import { Send, Square } from "lucide-react";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useStreamCompletion } from "@/src/hooks/useStreamCompletion";
import { useTeamStreamCompletion } from "@/src/hooks/useTeamStreamCompletion";
import { useTranslation } from "react-i18next";
import { api } from "@/src/lib/tauri";
import type { TeamMemberRow } from "@/src/lib/tauri";

interface ChatInputProps {
  isTeamSession?: boolean;
  teamId?: string | null;
}

export function ChatInput({ isTeamSession = false, teamId = null }: ChatInputProps) {
  const [input, setInput] = useState("");
  const [teamMembers, setTeamMembers] = useState<TeamMemberRow[]>([]);
  const { sendMessage, isStreaming } = useStreamCompletion();
  const { sendTeamMessage, isTeamStreaming } = useTeamStreamCompletion();
  const { t } = useTranslation();

  const busy = isStreaming || isTeamStreaming;

  // Load team members when in team mode
  useEffect(() => {
    if (isTeamSession && teamId) {
      api.getTeam(teamId).then(([, members]) => {
        setTeamMembers(members);
      }).catch(console.error);
    } else {
      setTeamMembers([]);
    }
  }, [isTeamSession, teamId]);

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || busy) return;

    if (isTeamSession && teamMembers.length > 0) {
      sendTeamMessage(trimmed, teamMembers);
    } else {
      sendMessage(trimmed);
    }
    setInput("");
  };

  return (
    <div className="flex items-end gap-2 px-4 py-3 border-t" style={{ borderColor: "var(--glass-border)" }}>
      <div className="flex-1">
        <GlassInput
          value={input}
          onChange={setInput}
          placeholder={t("chat.placeholder")}
          multiline
          onSubmit={handleSend}
          disabled={busy}
        />
      </div>
      {busy ? (
        <GlassButton variant="secondary" icon={<Square size={18} />} title="Stop" />
      ) : (
        <GlassButton
          variant="primary"
          icon={<Send size={18} />}
          onClick={handleSend}
          disabled={!input.trim()}
          title={t("chat.send")}
        />
      )}
    </div>
  );
}
