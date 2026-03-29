import { ReactNode } from "react";

interface GlassCardProps {
  children: ReactNode;
  active?: boolean;
  onClick?: () => void;
  className?: string;
}

export function GlassCard({ children, active = false, onClick, className = "" }: GlassCardProps) {
  return (
    <div
      onClick={onClick}
      className={`px-3 py-2 cursor-pointer transition-all duration-150
        hover:brightness-110 rounded-[var(--radius)] ${className}`}
      style={{
        background: active ? "var(--glass-bg)" : "transparent",
        border: active ? "1px solid var(--accent)" : "1px solid transparent",
      }}
    >
      {children}
    </div>
  );
}
