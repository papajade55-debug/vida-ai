import { useState, useEffect } from "react";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { StatusDot } from "@/src/design-system/StatusDot";
import { api } from "@/src/lib/tauri";
import { Copy, RefreshCw } from "lucide-react";

export function RemoteSettings() {
  const [enabled, setEnabled] = useState(false);
  const [port, setPort] = useState("3690");
  const [token, setToken] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadStatus();
  }, []);

  async function loadStatus() {
    try {
      const status = await api.getRemoteStatus();
      setEnabled(status.running);
      if (status.port) setPort(String(status.port));
      const t = await api.getRemoteToken();
      setToken(t);
    } catch (e) {
      console.error("Failed to load remote status:", e);
    } finally {
      setLoading(false);
    }
  }

  async function handleToggle() {
    try {
      if (enabled) {
        await api.disableRemote();
      } else {
        await api.enableRemote(parseInt(port));
      }
      setEnabled(!enabled);
    } catch (e) {
      console.error("Failed to toggle remote:", e);
    }
  }

  async function handleRegenerate() {
    try {
      const newToken = await api.regenerateRemoteToken();
      setToken(newToken);
    } catch (e) {
      console.error("Failed to regenerate token:", e);
    }
  }

  function handleCopy() {
    navigator.clipboard.writeText(token);
  }

  if (loading) {
    return <div style={{ color: "var(--text-secondary)", padding: 16 }}>Loading...</div>;
  }

  return (
    <div className="flex flex-col gap-4">
      {/* Status + Toggle */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <StatusDot status={enabled ? "idle" : "offline"} />
          <span className="text-sm" style={{ color: "var(--text-primary)" }}>
            {enabled ? "Server running" : "Server stopped"}
          </span>
        </div>
        <GlassButton variant={enabled ? "secondary" : "primary"} onClick={handleToggle}>
          {enabled ? "Disable" : "Enable"}
        </GlassButton>
      </div>

      {/* Port */}
      <div>
        <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
          Port
        </label>
        <GlassInput
          value={port}
          onChange={setPort}
          placeholder="3690"
          disabled={enabled}
        />
      </div>

      {/* URL */}
      {enabled && (
        <div
          className="text-xs px-3 py-2 rounded-lg"
          style={{ background: "var(--glass-bg)", color: "var(--accent)" }}
        >
          http://localhost:{port}/api/health
        </div>
      )}

      {/* Token */}
      <div>
        <label className="text-xs font-medium mb-1 block" style={{ color: "var(--text-secondary)" }}>
          API Token
        </label>
        <div className="flex gap-2">
          <div
            className="flex-1 text-xs px-3 py-2 rounded-lg overflow-hidden text-ellipsis"
            style={{
              background: "var(--glass-bg)",
              border: "1px solid var(--glass-border)",
              color: "var(--text-primary)",
              fontFamily: "monospace",
            }}
          >
            {token || "No token generated"}
          </div>
          <GlassButton variant="ghost" icon={<Copy size={14} />} onClick={handleCopy} />
          <GlassButton variant="ghost" icon={<RefreshCw size={14} />} onClick={handleRegenerate} />
        </div>
      </div>
    </div>
  );
}
