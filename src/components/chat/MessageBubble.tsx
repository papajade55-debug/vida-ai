import { memo, useState, useCallback, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import remarkGfm from "remark-gfm";
import { Copy, Check, ChevronDown, ChevronRight, Wrench } from "lucide-react";
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

// ── Tool call block (collapsible) ──

interface ToolCallData {
  name: string;
  args: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

function ToolCallBlock({ toolCall }: { toolCall: ToolCallData }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className="my-2 rounded-lg overflow-hidden text-xs"
      style={{
        background: "var(--glass-bg)",
        border: "1px solid var(--glass-border)",
      }}
    >
      {/* Header - always visible */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 px-3 py-2 hover:opacity-80 transition-opacity"
        style={{
          background: toolCall.isError ? "#ef444415" : "var(--accent-10)",
          color: toolCall.isError ? "#ef4444" : "var(--accent)",
          border: "none",
          cursor: "pointer",
        }}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <Wrench size={12} />
        <span className="font-mono font-medium">{toolCall.name}</span>
        {toolCall.isError && (
          <span className="ml-auto text-[10px] px-1.5 py-0.5 rounded bg-red-500/20 text-red-400">
            error
          </span>
        )}
      </button>

      {/* Expandable body */}
      {expanded && (
        <div className="px-3 py-2 space-y-2" style={{ borderTop: "1px solid var(--glass-border)" }}>
          {/* Arguments */}
          <div>
            <span className="font-medium" style={{ color: "var(--text-secondary)" }}>
              Arguments:
            </span>
            <pre
              className="mt-1 p-2 rounded text-[11px] font-mono overflow-x-auto"
              style={{ background: "var(--bg-primary)", color: "var(--text-primary)" }}
            >
              {JSON.stringify(toolCall.args, null, 2)}
            </pre>
          </div>

          {/* Result */}
          {toolCall.result !== undefined && (
            <div>
              <span className="font-medium" style={{ color: "var(--text-secondary)" }}>
                Result:
              </span>
              <pre
                className="mt-1 p-2 rounded text-[11px] font-mono overflow-x-auto max-h-48 overflow-y-auto"
                style={{
                  background: "var(--bg-primary)",
                  color: toolCall.isError ? "#ef4444" : "var(--text-primary)",
                }}
              >
                {toolCall.result}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Parse tool calls from message content ──

const TOOL_CALL_REGEX = /<tool_call>\s*(\{[\s\S]*?\})\s*<\/tool_call>/g;
const TOOL_RESULT_REGEX = /<tool_result\s*(?:name="([^"]*)")?\s*(?:error="([^"]*)")?>\s*([\s\S]*?)\s*<\/tool_result>/g;

interface ContentSegment {
  type: "text" | "tool_call";
  content: string;
  toolCall?: ToolCallData;
}

function parseToolCalls(content: string): ContentSegment[] {
  // First, collect tool results by name
  const results = new Map<string, { result: string; isError: boolean }>();
  let resultMatch;
  const resultRegex = new RegExp(TOOL_RESULT_REGEX.source, "g");
  while ((resultMatch = resultRegex.exec(content)) !== null) {
    const name = resultMatch[1] || "unknown";
    const isError = resultMatch[2] === "true";
    const result = resultMatch[3];
    results.set(name, { result, isError });
  }

  // Check if there are any tool_call tags
  const hasToolCalls = /<tool_call>/.test(content);
  if (!hasToolCalls && results.size === 0) {
    return [{ type: "text", content }];
  }

  const segments: ContentSegment[] = [];
  let lastIndex = 0;
  const combinedRegex = /<tool_call>\s*(\{[\s\S]*?\})\s*<\/tool_call>|<tool_result\s*(?:name="[^"]*")?\s*(?:error="[^"]*")?>\s*[\s\S]*?\s*<\/tool_result>/g;

  let match;
  while ((match = combinedRegex.exec(content)) !== null) {
    // Add text before this match
    if (match.index > lastIndex) {
      const textBefore = content.slice(lastIndex, match.index).trim();
      if (textBefore) {
        segments.push({ type: "text", content: textBefore });
      }
    }

    // Check if it's a tool_call
    if (match[1]) {
      try {
        const parsed = JSON.parse(match[1]);
        const toolName = parsed.name || "unknown";
        const toolArgs = parsed.args || parsed.arguments || {};
        const toolResult = results.get(toolName);

        segments.push({
          type: "tool_call",
          content: "",
          toolCall: {
            name: toolName,
            args: toolArgs,
            result: toolResult?.result,
            isError: toolResult?.isError,
          },
        });
      } catch {
        // If JSON parse fails, show as text
        segments.push({ type: "text", content: match[0] });
      }
    }
    // tool_result tags are consumed by the tool_call they belong to (or hidden)

    lastIndex = match.index + match[0].length;
  }

  // Remaining text after last match
  if (lastIndex < content.length) {
    const remaining = content.slice(lastIndex).trim();
    if (remaining) {
      segments.push({ type: "text", content: remaining });
    }
  }

  return segments.length > 0 ? segments : [{ type: "text", content }];
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

    // Parse tool calls from assistant messages
    const segments = useMemo(
      () => (isUser ? null : parseToolCalls(displayContent)),
      [isUser, displayContent]
    );

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
            {segments?.map((segment, i) =>
              segment.type === "tool_call" && segment.toolCall ? (
                <ToolCallBlock key={`tc-${i}`} toolCall={segment.toolCall} />
              ) : (
                <ReactMarkdown
                  key={`md-${i}`}
                  rehypePlugins={rehypePlugins}
                  remarkPlugins={remarkPlugins}
                  components={mdComponents}
                >
                  {segment.content}
                </ReactMarkdown>
              )
            )}
            {isStreaming && <span className="streaming-cursor" />}
          </div>
          {time && <span className="msg-time msg-time--assistant">{time}</span>}
        </div>
      </div>
    );
  },
  (prev, next) => {
    if (prev.message.id !== next.message.id) return false;
    if (prev.message.content !== next.message.content) return false;
    return true;
  }
);
