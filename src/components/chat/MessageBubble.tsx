import { memo, useState, useCallback, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import remarkGfm from "remark-gfm";
import { Copy, Check } from "lucide-react";
import { useStore } from "@/src/stores/store";
import type { MessageRow } from "@/src/lib/tauri";
import type { Components } from "react-markdown";

// ── Copy button for code blocks ──

function CopyButton({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [code]);

  return (
    <button
      onClick={handleCopy}
      className="copy-btn"
      aria-label="Copier le code"
    >
      {copied ? <Check size={14} /> : <Copy size={14} />}
    </button>
  );
}

// ── Markdown components override ──

function extractText(children: React.ReactNode): string {
  if (typeof children === "string") return children;
  if (Array.isArray(children)) return children.map(extractText).join("");
  if (children && typeof children === "object" && "props" in children) {
    const el = children as React.ReactElement<{ children?: React.ReactNode }>;
    return extractText(el.props.children);
  }
  return String(children ?? "");
}

const mdComponents: Components = {
  pre({ children }) {
    const code = extractText(children);
    return (
      <div className="code-block-wrapper">
        <CopyButton code={code} />
        <pre>{children}</pre>
      </div>
    );
  },
};

// ── Rehype / remark plugins (stable refs) ──

const rehypePlugins = [rehypeHighlight];
const remarkPlugins = [remarkGfm];

// ── Timestamp formatter ──

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

// ── MessageBubble ──

interface MessageBubbleProps {
  message: MessageRow;
}

export const MessageBubble = memo(
  function MessageBubble({ message }: MessageBubbleProps) {
    const streamingMessageId = useStore((s) => s.streamingMessageId);
    const streamingContent = useStore((s) => s.streamingContent);
    const agentStreaming = useStore((s) => s.agentStreaming);

    const isUser = message.role === "user";
    const isSoloStreaming = message.id === streamingMessageId;
    const isAgentStreaming = message.agent_id !== null && message.agent_id in agentStreaming;
    const isStreaming = isSoloStreaming || isAgentStreaming;

    let displayContent = message.content;
    if (isSoloStreaming) {
      displayContent = streamingContent;
    } else if (isAgentStreaming && message.agent_id) {
      displayContent = agentStreaming[message.agent_id];
    }

    const time = useMemo(() => formatTime(message.created_at), [message.created_at]);

    if (isUser) {
      return (
        <div className="msg-row msg-row--user">
          <div className="msg-bubble msg-bubble--user">
            <p className="msg-text">{displayContent}</p>
            {time && <span className="msg-time msg-time--user">{time}</span>}
          </div>
        </div>
      );
    }

    // Assistant
    const agentName = message.agent_name;
    const agentColor = message.agent_color;

    return (
      <div className="msg-row msg-row--assistant">
        <div className="msg-bubble msg-bubble--assistant">
          {agentName && agentColor && (
            <div
              className="flex items-center gap-2 px-3 py-1.5 -mx-4 -mt-3 mb-2 rounded-t-[var(--radius)]"
              style={{
                background: `${agentColor}14`,
                borderBottom: `1px solid ${agentColor}30`,
              }}
            >
              <span
                className="inline-block w-2 h-2 rounded-full flex-shrink-0"
                style={{ backgroundColor: agentColor }}
              />
              <span className="text-xs font-medium" style={{ color: agentColor }}>
                {agentName}
              </span>
            </div>
          )}
          <div className="msg-markdown">
            <ReactMarkdown
              rehypePlugins={rehypePlugins}
              remarkPlugins={remarkPlugins}
              components={mdComponents}
            >
              {displayContent}
            </ReactMarkdown>
            {isStreaming && <span className="streaming-cursor" />}
          </div>
          {time && <span className="msg-time msg-time--assistant">{time}</span>}
        </div>
      </div>
    );
  },
  (prev, next) => {
    // Re-render only if content changed or streaming state changed
    if (prev.message.id !== next.message.id) return false;
    if (prev.message.content !== next.message.content) return false;
    return true;
  }
);
