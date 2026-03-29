import type { ReactNode } from "react";

interface GlassPanelProps {
  children: ReactNode;
  className?: string;
  padding?: string;
}

export function GlassPanel({
  children,
  className = "",
  padding = "p-4",
}: GlassPanelProps) {
  return (
    <div
      className={`rounded-[var(--radius-lg)] ${padding} ${className}`}
      style={{
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        WebkitBackdropFilter: "blur(var(--glass-blur))",
        border: "1px solid var(--glass-border)",
        boxShadow: "var(--glass-shadow)",
      }}
    >
      {children}
    </div>
  );
}
