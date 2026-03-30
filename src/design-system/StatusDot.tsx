import { motion } from "motion/react";

type Status = "idle" | "streaming" | "error" | "offline" | "unknown";

interface StatusDotProps {
  status: Status;
  className?: string;
}

const statusColors: Record<Status, string> = {
  idle: "var(--status-active)",
  streaming: "var(--status-streaming)",
  error: "var(--status-error)",
  offline: "var(--status-offline)",
  unknown: "var(--text-secondary, #6b7280)",
};

export function StatusDot({ status, className = "" }: StatusDotProps) {
  const color = statusColors[status];

  return (
    <motion.span
      data-no-theme-transition
      className={`inline-block rounded-full ${className}`}
      style={{
        width: 8,
        height: 8,
        backgroundColor: color,
        boxShadow: status !== "offline" ? `0 0 6px ${color}` : "none",
      }}
      animate={
        status === "streaming"
          ? { scale: [1, 1.4, 1], opacity: [1, 0.7, 1] }
          : { scale: 1, opacity: 1 }
      }
      transition={
        status === "streaming"
          ? { duration: 1.5, repeat: Infinity, ease: "easeInOut" }
          : { duration: 0.2 }
      }
    />
  );
}
