import { useEffect } from "react";
import { useStore } from "@/src/stores/store";

export function useTheme() {
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    // Only set from OS if no persisted preference
    const persisted = localStorage.getItem("vida-store");
    if (!persisted) {
      setTheme(mq.matches ? "dark" : "light");
    }
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem("vida-store")) {
        setTheme(e.matches ? "dark" : "light");
      }
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [setTheme]);

  const toggleTheme = () => setTheme(theme === "dark" ? "light" : "dark");

  return { theme, toggleTheme };
}
