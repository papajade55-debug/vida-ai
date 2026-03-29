import { useState } from "react";
import { Send, Square } from "lucide-react";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useStreamCompletion } from "@/src/hooks/useStreamCompletion";
import { useTranslation } from "react-i18next";

export function ChatInput() {
  const [input, setInput] = useState("");
  const { sendMessage, isStreaming } = useStreamCompletion();
  const { t } = useTranslation();

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || isStreaming) return;
    sendMessage(trimmed);
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
          disabled={isStreaming}
        />
      </div>
      {isStreaming ? (
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
