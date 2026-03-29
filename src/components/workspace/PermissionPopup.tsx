import { useState, useEffect } from "react";
import { Shield, Check, X } from "lucide-react";
import { motion, AnimatePresence } from "motion/react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GlassButton } from "@/src/design-system/GlassButton";

interface PermissionRequest {
  request_id: string;
  action: string;
  path?: string;
  description: string;
}

export function PermissionPopup() {
  const [request, setRequest] = useState<PermissionRequest | null>(null);

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("permission-request", (event) => {
      setRequest(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleRespond = async (allowed: boolean) => {
    if (!request) return;
    try {
      await invoke("respond_permission", {
        requestId: request.request_id,
        allowed,
      });
    } catch (e) {
      console.error("Failed to respond to permission request:", e);
    }
    setRequest(null);
  };

  return (
    <AnimatePresence>
      {request && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-[100]"
            style={{ background: "rgba(0, 0, 0, 0.4)" }}
          />

          {/* Popup */}
          <motion.div
            initial={{ opacity: 0, scale: 0.9, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.9, y: 20 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="fixed inset-0 z-[100] flex items-center justify-center p-4 pointer-events-none"
          >
            <div
              className="w-full max-w-sm rounded-[var(--radius-lg)] overflow-hidden pointer-events-auto"
              style={{
                background: "var(--bg-secondary)",
                border: "1px solid var(--glass-border)",
                boxShadow: "var(--glass-shadow)",
              }}
            >
              {/* Header */}
              <div
                className="flex items-center gap-2 px-4 py-3 border-b"
                style={{ borderColor: "var(--glass-border)" }}
              >
                <Shield size={18} style={{ color: "var(--accent)" }} />
                <h3
                  className="text-sm font-semibold"
                  style={{ color: "var(--text-primary)" }}
                >
                  Permission Required
                </h3>
              </div>

              {/* Body */}
              <div className="px-4 py-3 space-y-2">
                <p
                  className="text-sm"
                  style={{ color: "var(--text-primary)" }}
                >
                  {request.description}
                </p>
                {request.path && (
                  <p
                    className="text-xs font-mono truncate"
                    style={{ color: "var(--text-secondary)" }}
                  >
                    {request.path}
                  </p>
                )}
                <p
                  className="text-xs"
                  style={{ color: "var(--text-secondary)" }}
                >
                  Action: {request.action}
                </p>
              </div>

              {/* Actions */}
              <div
                className="flex gap-2 px-4 py-3 border-t"
                style={{ borderColor: "var(--glass-border)" }}
              >
                <GlassButton
                  variant="ghost"
                  onClick={() => handleRespond(false)}
                  className="flex-1 !text-xs"
                  icon={<X size={14} />}
                >
                  Deny
                </GlassButton>
                <GlassButton
                  variant="primary"
                  onClick={() => handleRespond(true)}
                  className="flex-1 !text-xs"
                  icon={<Check size={14} />}
                >
                  Allow
                </GlassButton>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
