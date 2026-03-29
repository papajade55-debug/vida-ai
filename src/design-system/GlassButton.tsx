import { ReactNode, ButtonHTMLAttributes } from "react";

interface GlassButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost";
  icon?: ReactNode;
  children?: ReactNode;
}

const variantStyles = {
  primary: {
    background: "var(--accent)",
    color: "var(--accent-text)",
    border: "1px solid transparent",
  },
  secondary: {
    background: "var(--glass-bg)",
    color: "var(--text-primary)",
    border: "1px solid var(--glass-border)",
  },
  ghost: {
    background: "transparent",
    color: "var(--text-secondary)",
    border: "1px solid transparent",
  },
};

export function GlassButton({
  variant = "secondary",
  icon,
  children,
  className = "",
  disabled,
  ...props
}: GlassButtonProps) {
  return (
    <button
      className={`flex items-center gap-2 px-3 py-2 rounded-[var(--radius)] cursor-pointer
        hover:opacity-80 active:scale-95 transition-all duration-150
        disabled:opacity-40 disabled:cursor-not-allowed disabled:active:scale-100
        ${className}`}
      style={variantStyles[variant]}
      disabled={disabled}
      {...props}
    >
      {icon}
      {children}
    </button>
  );
}
