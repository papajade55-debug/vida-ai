import { useStore } from "@/src/stores/store";
import { useTranslation } from "react-i18next";

const LANGUAGES = [
  { code: "en", label: "English" },
  { code: "fr", label: "Francais" },
  { code: "zh-CN", label: "Chinese (Simplified)" },
];

const THEMES = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function GeneralSettings() {
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);
  const { t, i18n } = useTranslation();

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
  };

  return (
    <div className="space-y-6">
      {/* Language */}
      <div>
        <label
          className="block text-sm font-medium mb-2"
          style={{ color: "var(--text-primary)" }}
        >
          Language
        </label>
        <select
          value={i18n.language}
          onChange={(e) => handleLanguageChange(e.target.value)}
          className="w-full px-3 py-2 rounded-[var(--radius)] outline-none"
          style={{
            background: "var(--glass-bg)",
            color: "var(--text-primary)",
            border: "1px solid var(--glass-border)",
          }}
        >
          {LANGUAGES.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.label}
            </option>
          ))}
        </select>
      </div>

      {/* Theme */}
      <div>
        <label
          className="block text-sm font-medium mb-2"
          style={{ color: "var(--text-primary)" }}
        >
          Theme
        </label>
        <div className="flex gap-2">
          {THEMES.map((t) => (
            <button
              key={t.value}
              onClick={() => setTheme(t.value as "light" | "dark")}
              className="flex-1 px-4 py-2 rounded-[var(--radius)] text-sm font-medium cursor-pointer transition-all"
              style={{
                background:
                  theme === t.value ? "var(--accent)" : "var(--glass-bg)",
                color:
                  theme === t.value
                    ? "var(--accent-text)"
                    : "var(--text-primary)",
                border: `1px solid ${theme === t.value ? "transparent" : "var(--glass-border)"}`,
              }}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
