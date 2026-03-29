import { ReactNode } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore } from "@/src/stores/store";
import { Sidebar } from "./Sidebar";
import { PermissionPopup } from "@/src/components/workspace/PermissionPopup";

interface AppLayoutProps {
  children: ReactNode;
}

export function AppLayout({ children }: AppLayoutProps) {
  const sidebarOpen = useStore((s) => s.sidebarOpen);

  return (
    <div className="flex h-screen w-screen overflow-hidden" style={{ background: "var(--bg-primary)" }}>
      <AnimatePresence>
        {sidebarOpen && (
          <motion.div
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: "var(--sidebar-width)", opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="h-full overflow-hidden flex-shrink-0"
          >
            <Sidebar />
          </motion.div>
        )}
      </AnimatePresence>
      <main className="flex-1 h-full overflow-hidden">
        {children}
      </main>
      <PermissionPopup />
    </div>
  );
}
