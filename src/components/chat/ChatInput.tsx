import { useState, useEffect } from "react";
import { Send, Square, Paperclip } from "lucide-react";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useStreamCompletion } from "@/src/hooks/useStreamCompletion";
import { useTeamStreamCompletion } from "@/src/hooks/useTeamStreamCompletion";
import { useTranslation } from "react-i18next";
import { api } from "@/src/lib/tauri";
import { useStore } from "@/src/stores/store";
import { FilePreview, type AttachedFile } from "./FilePreview";
import type { TeamMemberRow } from "@/src/lib/tauri";

interface ChatInputProps {
  isTeamSession?: boolean;
  teamId?: string | null;
  attachedFiles?: AttachedFile[];
  onRemoveFile?: (index: number) => void;
  onClearFiles?: () => void;
}

export function ChatInput({
  isTeamSession = false,
  teamId = null,
  attachedFiles = [],
  onRemoveFile,
  onClearFiles,
}: ChatInputProps) {
  const [input, setInput] = useState("");
  const [teamMembers, setTeamMembers] = useState<TeamMemberRow[]>([]);
  const [sendingVision, setSendingVision] = useState(false);
  const { sendMessage, isStreaming } = useStreamCompletion();
  const { sendTeamMessage, isTeamStreaming } = useTeamStreamCompletion();
  const currentSessionId = useStore((s) => s.currentSessionId);
  const addMessage = useStore((s) => s.addMessage);
  const { t } = useTranslation();

  const busy = isStreaming || isTeamStreaming || sendingVision;

  // Load team members when in team mode
  useEffect(() => {
    if (isTeamSession && teamId) {
      api
        .getTeam(teamId)
        .then(([, members]) => {
          setTeamMembers(members);
        })
        .catch(console.error);
    } else {
      setTeamMembers([]);
    }
  }, [isTeamSession, teamId]);

  const handleSend = async () => {
    const trimmed = input.trim();
    if (!trimmed || busy) return;

    // Check if there's an attached image -> use vision
    const imageFile = attachedFiles.find((f) => f.type.startsWith("image/"));

    if (imageFile && imageFile.dataUrl && currentSessionId) {
      setSendingVision(true);
      try {
        // Extract base64 data from dataUrl (strip "data:image/png;base64," prefix)
        const base64Data = imageFile.dataUrl.split(",")[1] || "";

        // Build prompt: include text file contents if any
        let fullPrompt = trimmed;
        for (const f of attachedFiles) {
          if (f.textContent) {
            fullPrompt += `\n\n--- ${f.name} ---\n${f.textContent}`;
          }
        }

        // Add user message to store
        addMessage(currentSessionId, {
          id: crypto.randomUUID(),
          session_id: currentSessionId,
          role: "user",
          content: fullPrompt,
          token_count: null,
          created_at: new Date().toISOString(),
          agent_id: null,
          agent_name: null,
          agent_color: null,
        });

        const response = await api.sendVisionMessage(
          currentSessionId,
          base64Data,
          fullPrompt
        );

        // Add assistant message
        addMessage(currentSessionId, {
          id: crypto.randomUUID(),
          session_id: currentSessionId,
          role: "assistant",
          content: response.content,
          token_count: response.total_tokens,
          created_at: new Date().toISOString(),
          agent_id: null,
          agent_name: null,
          agent_color: null,
        });
      } catch (err) {
        console.error("Vision message failed:", err);
      } finally {
        setSendingVision(false);
      }
      setInput("");
      onClearFiles?.();
      return;
    }

    // Build message content: include text file contents
    let content = trimmed;
    for (const f of attachedFiles) {
      if (f.textContent) {
        content += `\n\n--- ${f.name} ---\n\`\`\`\n${f.textContent}\n\`\`\``;
      }
    }

    if (isTeamSession && teamMembers.length > 0) {
      sendTeamMessage(content, teamMembers);
    } else {
      sendMessage(content);
    }
    setInput("");
    onClearFiles?.();
  };

  return (
    <div
      className="px-4 py-3 border-t"
      style={{ borderColor: "var(--glass-border)" }}
    >
      {/* Attached files preview */}
      {attachedFiles.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-2">
          {attachedFiles.map((file, index) => (
            <FilePreview
              key={`${file.name}-${index}`}
              file={file}
              onRemove={() => onRemoveFile?.(index)}
            />
          ))}
        </div>
      )}

      {/* Input row */}
      <div className="flex items-end gap-2">
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
          <GlassButton
            variant="secondary"
            icon={<Square size={18} />}
            title="Stop"
          />
        ) : (
          <GlassButton
            variant="primary"
            icon={<Send size={18} />}
            onClick={handleSend}
            disabled={!input.trim() && attachedFiles.length === 0}
            title={t("chat.send")}
          />
        )}
      </div>
    </div>
  );
}
