import { useRef, useEffect, useState } from "react";
import { useStore } from "@/src/stores/store";
import { MessageBubble } from "./MessageBubble";
import { GlassButton } from "@/src/design-system/GlassButton";
import { ArrowDown } from "lucide-react";

export function MessageList() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const messages = useStore((s) => (currentSessionId ? s.messages[currentSessionId] || [] : []));
  const streamingContent = useStore((s) => s.streamingContent);

  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [showScrollBtn, setShowScrollBtn] = useState(false);

  // Auto-scroll on new messages or streaming
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [messages.length, streamingContent, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const atBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(atBottom);
    setShowScrollBtn(!atBottom);
  };

  const scrollToBottom = () => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
      setAutoScroll(true);
      setShowScrollBtn(false);
    }
  };

  if (messages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <div className="text-2xl mb-2">💬</div>
          <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
            Send a message to begin
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 relative overflow-hidden">
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="h-full overflow-y-auto px-4 py-4"
      >
        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
      </div>
      {showScrollBtn && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2">
          <GlassButton variant="secondary" icon={<ArrowDown size={16} />} onClick={scrollToBottom}>
            ↓
          </GlassButton>
        </div>
      )}
    </div>
  );
}
