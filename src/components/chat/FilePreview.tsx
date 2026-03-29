import { X, FileText, Image } from "lucide-react";

export interface AttachedFile {
  name: string;
  type: string;
  size: number;
  /** base64-encoded data (for images) */
  dataUrl?: string;
  /** raw text content (for text files) */
  textContent?: string;
}

interface FilePreviewProps {
  file: AttachedFile;
  onRemove: () => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function FilePreview({ file, onRemove }: FilePreviewProps) {
  const isImage = file.type.startsWith("image/");

  return (
    <div
      className="relative inline-flex items-center gap-2 px-3 py-2 rounded-lg max-w-[200px]"
      style={{
        background: "var(--glass-bg)",
        border: "1px solid var(--glass-border)",
      }}
    >
      {isImage && file.dataUrl ? (
        <img
          src={file.dataUrl}
          alt={file.name}
          className="w-10 h-10 rounded object-cover flex-shrink-0"
        />
      ) : (
        <div
          className="w-10 h-10 rounded flex items-center justify-center flex-shrink-0"
          style={{ background: "var(--glass-border)" }}
        >
          {isImage ? (
            <Image size={18} style={{ color: "var(--text-secondary)" }} />
          ) : (
            <FileText size={18} style={{ color: "var(--text-secondary)" }} />
          )}
        </div>
      )}

      <div className="min-w-0 flex-1">
        <div
          className="text-xs font-medium truncate"
          style={{ color: "var(--text-primary)" }}
        >
          {file.name}
        </div>
        <div className="text-[10px]" style={{ color: "var(--text-secondary)" }}>
          {formatSize(file.size)}
        </div>
      </div>

      <button
        onClick={onRemove}
        className="absolute -top-1.5 -right-1.5 w-5 h-5 rounded-full flex items-center justify-center cursor-pointer hover:scale-110 transition-transform"
        style={{
          background: "var(--status-error)",
          color: "#fff",
        }}
      >
        <X size={12} />
      </button>
    </div>
  );
}
