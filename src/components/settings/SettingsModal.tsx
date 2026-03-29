import { useState } from "react";
import { X, Settings, Shield, Cpu, FolderOpen } from "lucide-react";
import { motion, AnimatePresence } from "motion/react";
import { useStore } from "@/src/stores/store";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GeneralSettings } from "./GeneralSettings";
import { SecuritySettings } from "./SecuritySettings";
import { ProviderSettings } from "./ProviderSettings";
import { WorkspaceSettings } from "./WorkspaceSettings";

type SettingsTab = "general" | "security" | "providers" | "workspace";

const TABS: { id: SettingsTab; label: string; icon: typeof Settings }[] = [
  { id: "general", label: "General", icon: Settings },
  { id: "workspace", label: "Workspace", icon: FolderOpen },
  { id: "security", label: "Security", icon: Shield },
  { id: "providers", label: "Providers", icon: Cpu },
];

export function SettingsModal() {
  const settingsOpen = useStore((s) => s.settingsOpen);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");

  return (
    <AnimatePresence>
      {settingsOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50"
            style={{ background: "rgba(0, 0, 0, 0.5)" }}
            onClick={() => setSettingsOpen(false)}
          />

          {/* Modal */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 20 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4 pointer-events-none"
          >
            <div
              className="w-full max-w-lg max-h-[80vh] rounded-[var(--radius-lg)] overflow-hidden flex flex-col pointer-events-auto"
              style={{
                background: "var(--bg-secondary)",
                border: "1px solid var(--glass-border)",
                boxShadow: "var(--glass-shadow)",
              }}
            >
              {/* Header */}
              <div
                className="flex items-center justify-between px-5 py-4 border-b"
                style={{ borderColor: "var(--glass-border)" }}
              >
                <h2
                  className="text-lg font-semibold"
                  style={{ color: "var(--text-primary)" }}
                >
                  Settings
                </h2>
                <GlassButton
                  variant="ghost"
                  icon={<X size={18} />}
                  onClick={() => setSettingsOpen(false)}
                  className="!px-2 !py-1"
                />
              </div>

              {/* Tabs */}
              <div
                className="flex border-b px-5"
                style={{ borderColor: "var(--glass-border)" }}
              >
                {TABS.map((tab) => {
                  const Icon = tab.icon;
                  const isActive = activeTab === tab.id;
                  return (
                    <button
                      key={tab.id}
                      onClick={() => setActiveTab(tab.id)}
                      className="flex items-center gap-1.5 px-3 py-2.5 text-sm font-medium cursor-pointer transition-colors relative"
                      style={{
                        color: isActive
                          ? "var(--accent)"
                          : "var(--text-secondary)",
                      }}
                    >
                      <Icon size={14} />
                      {tab.label}
                      {isActive && (
                        <motion.div
                          layoutId="settings-tab-indicator"
                          className="absolute bottom-0 left-0 right-0 h-0.5"
                          style={{ background: "var(--accent)" }}
                        />
                      )}
                    </button>
                  );
                })}
              </div>

              {/* Content */}
              <div className="flex-1 overflow-y-auto px-5 py-4">
                {activeTab === "general" && <GeneralSettings />}
                {activeTab === "workspace" && <WorkspaceSettings />}
                {activeTab === "security" && <SecuritySettings />}
                {activeTab === "providers" && <ProviderSettings />}
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
