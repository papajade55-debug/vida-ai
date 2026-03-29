import { useState, useCallback, DragEvent } from "react";
import { Upload, MessageSquarePlus } from "lucide-react";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useStore } from "@/src/stores/store";
import { api } from "@/src/lib/tauri";
import { ChatHeader } from "./ChatHeader";
import { MessageList } from "./MessageList";
import { ChatInput } from "./ChatInput";
import type { AttachedFile } from "./FilePreview";

const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];
const TEXT_EXTENSIONS = [".txt", ".md", ".py", ".rs", ".ts", ".tsx", ".js", ".jsx", ".json", ".yaml", ".yml", ".toml", ".sh", ".css", ".html", ".xml", ".csv", ".log"];

function isTextFile(file: File): boolean {
  if (file.type.startsWith("text/")) return true;
  return TEXT_EXTENSIONS.some((ext) => file.name.toLowerCase().endsWith(ext));
}

export function ChatArea() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const sessions = useStore((s) => s.sessions);
  const [isDragging, setIsDragging] = useState(false);
  const [attachedFiles, setAttachedFiles] = useState<AttachedFile[]>([]);

  const currentSession = sessions.find((s) => s.id === currentSessionId);
  const isTeamSession = currentSession?.team_id != null;

  const processFile = useCallback((file: File) => {
    if (IMAGE_TYPES.includes(file.type)) {
      const reader = new FileReader();
      reader.onload = () => {
        setAttachedFiles((prev) => [
          ...prev,
          {
            name: file.name,
            type: file.type,
            size: file.size,
            dataUrl: reader.result as string,
          },
        ]);
      };
      reader.readAsDataURL(file);
    } else if (isTextFile(file)) {
      const reader = new FileReader();
      reader.onload = () => {
        setAttachedFiles((prev) => [
          ...prev,
          {
            name: file.name,
            type: file.type || "text/plain",
            size: file.size,
            textContent: reader.result as string,
          },
        ]);
      };
      reader.readAsText(file);
    }
  }, []);

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback(
    (e: DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);

      const files = Array.from(e.dataTransfer.files);
      for (const file of files) {
        processFile(file);
      }
    },
    [processFile]
  );

  const removeFile = useCallback((index: number) => {
    setAttachedFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const clearFiles = useCallback(() => {
    setAttachedFiles([]);
  }, []);

  const addSession = useStore((s) => s.addSession);
  const setCurrentSession = useStore((s) => s.setCurrentSession);
  const setMessages = useStore((s) => s.setMessages);

  const [chatError, setChatError] = useState<string | null>(null);

  const handleStartChat = async () => {
    setChatError("Creating session...");
    try {
      const session = await api.createSession("ollama", "qwen3:14b");
      addSession(session);
      setCurrentSession(session.id);
      setMessages(session.id, []);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setChatError("Error: " + msg);
    }
  };

  if (!currentSessionId) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="text-4xl mb-4">🤖</div>
          <div className="text-xl font-semibold mb-2" style={{ color: "var(--text-primary)" }}>
            Welcome to Vida AI
          </div>
          <div className="text-sm mb-6" style={{ color: "var(--text-secondary)" }}>
            Create a new chat to get started
          </div>
          <GlassButton
            variant="primary"
            icon={<MessageSquarePlus size={18} />}
            onClick={handleStartChat}
          >
            New Chat
          </GlassButton>
          {chatError && (
            <div className="mt-3 text-xs px-3 py-2 rounded-lg max-w-md" style={{ background: "rgba(239,68,68,0.15)", color: "var(--status-error)" }}>
              {chatError}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <GlassPanel className="h-full flex flex-col overflow-hidden relative" padding="p-0">
      <div
        className="flex flex-col flex-1 overflow-hidden"
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <ChatHeader />
        <MessageList />
        <ChatInput
          isTeamSession={isTeamSession}
          teamId={currentSession?.team_id ?? null}
          attachedFiles={attachedFiles}
          onRemoveFile={removeFile}
          onClearFiles={clearFiles}
        />
      </div>

      {/* Drop zone overlay */}
      {isDragging && (
        <div
          className="absolute inset-0 z-50 flex items-center justify-center rounded-[var(--radius-lg)]"
          style={{
            background: "rgba(var(--accent-rgb, 99, 102, 241), 0.15)",
            border: "2px dashed var(--accent)",
            backdropFilter: "blur(4px)",
          }}
        >
          <div className="flex flex-col items-center gap-2">
            <Upload size={48} style={{ color: "var(--accent)" }} />
            <span className="text-lg font-medium" style={{ color: "var(--accent)" }}>
              Drop files here
            </span>
          </div>
        </div>
      )}
    </GlassPanel>
  );
}
