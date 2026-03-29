import { useRef, useEffect, KeyboardEvent, ChangeEvent } from "react";

interface GlassInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  multiline?: boolean;
  onSubmit?: () => void;
  disabled?: boolean;
  className?: string;
}

export function GlassInput({
  value,
  onChange,
  placeholder,
  multiline = false,
  onSubmit,
  disabled = false,
  className = "",
}: GlassInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (multiline && textareaRef.current) {
      textareaRef.current.style.height = "auto";
      const scrollHeight = textareaRef.current.scrollHeight;
      const maxHeight = 8 * 24; // 8 lines * ~24px line-height
      textareaRef.current.style.height = `${Math.min(scrollHeight, maxHeight)}px`;
    }
  }, [value, multiline]);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && onSubmit) {
      e.preventDefault();
      onSubmit();
    }
  };

  const handleChange = (e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    onChange(e.target.value);
  };

  const baseStyle = {
    background: "var(--glass-bg)",
    color: "var(--text-primary)",
    border: "1px solid var(--glass-border)",
    borderRadius: "var(--radius)",
  };

  const baseClass = `w-full px-3 py-2 outline-none placeholder:text-[var(--text-secondary)]
    focus:border-[var(--accent)] transition-colors ${className}`;

  if (multiline) {
    return (
      <textarea
        ref={textareaRef}
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
        className={`${baseClass} resize-none overflow-y-auto`}
        style={{ ...baseStyle, maxHeight: "192px" }}
      />
    );
  }

  return (
    <input
      type="text"
      value={value}
      onChange={handleChange}
      onKeyDown={handleKeyDown}
      placeholder={placeholder}
      disabled={disabled}
      className={baseClass}
      style={baseStyle}
    />
  );
}
