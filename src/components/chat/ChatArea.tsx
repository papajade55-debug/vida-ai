import { GlassPanel } from "@/src/design-system/GlassPanel";
import { useStore } from "@/src/stores/store";
import { ChatHeader } from "./ChatHeader";
import { MessageList } from "./MessageList";
import { ChatInput } from "./ChatInput";

export function ChatArea() {
  const currentSessionId = useStore((s) => s.currentSessionId);

  if (!currentSessionId) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="text-4xl mb-4">🤖</div>
          <div className="text-xl font-semibold mb-2" style={{ color: "var(--text-primary)" }}>
            Welcome to Vida AI
          </div>
          <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
            Create a new chat to get started
          </div>
        </div>
      </div>
    );
  }

  return (
    <GlassPanel className="h-full flex flex-col overflow-hidden" padding="p-0">
      <ChatHeader />
      <MessageList />
      <ChatInput />
    </GlassPanel>
  );
}
