import { ReactNode, useMemo, useState } from "react";
import { ShieldCheck } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassPanel } from "@/src/design-system/GlassPanel";
import { useAuth } from "@/src/hooks/useAuth";

interface AuthGateProps {
  children: ReactNode;
}

const ROLE_OPTIONS = [
  { value: "architect", label: "Architecte" },
  { value: "operator", label: "Operateur" },
];

export function AuthGate({ children }: AuthGateProps) {
  const { authActor, hasUsers, loading, bootstrap, login } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const mode = useMemo(() => {
    if (loading || hasUsers === null) return "loading";
    return hasUsers ? "login" : "bootstrap";
  }, [hasUsers, loading]);

  if (authActor) {
    return <>{children}</>;
  }

  const handleSubmit = async () => {
    setSubmitting(true);
    setStatus(null);
    try {
      if (mode === "bootstrap") {
        if (password !== confirmPassword) {
          throw new Error("Passwords do not match");
        }
        await bootstrap(username.trim(), password);
      } else {
        await login(username.trim(), password);
      }
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      className="h-screen w-screen flex items-center justify-center p-6"
      style={{ background: "var(--bg-primary)" }}
    >
      <GlassPanel className="w-full max-w-md" padding="p-6">
        <div className="flex items-center gap-3 mb-5">
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center"
            style={{ background: "var(--accent)", color: "var(--accent-text)" }}
          >
            <ShieldCheck size={20} />
          </div>
          <div>
            <h1 className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
              Vida Access Control
            </h1>
            <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
              {mode === "bootstrap"
                ? "Bootstrap the first local Super Admin"
                : "Authenticate to unlock the control plane"}
            </p>
          </div>
        </div>

        <div className="space-y-3">
          <div>
            <label className="block text-xs mb-1" style={{ color: "var(--text-secondary)" }}>
              Username
            </label>
            <GlassInput value={username} onChange={setUsername} placeholder="admin.local" />
          </div>
          <div>
            <label className="block text-xs mb-1" style={{ color: "var(--text-secondary)" }}>
              Password
            </label>
            <GlassInput value={password} onChange={setPassword} placeholder="At least 8 characters" />
          </div>
          {mode === "bootstrap" && (
            <div>
              <label className="block text-xs mb-1" style={{ color: "var(--text-secondary)" }}>
                Confirm password
              </label>
              <GlassInput
                value={confirmPassword}
                onChange={setConfirmPassword}
                placeholder="Repeat password"
              />
            </div>
          )}
        </div>

        {mode === "bootstrap" && (
          <div
            className="mt-4 rounded-[var(--radius)] px-3 py-2 text-xs"
            style={{
              background: "var(--glass-bg)",
              border: "1px solid var(--glass-border)",
              color: "var(--text-secondary)",
            }}
          >
            The first account is always created as <strong>Super Admin</strong>. Additional
            users can then be created from Security settings with roles:
            {" "}
            {ROLE_OPTIONS.map((option) => option.label).join(", ")}.
          </div>
        )}

        {status && (
          <div
            className="mt-4 rounded-[var(--radius)] px-3 py-2 text-xs"
            style={{
              background: "rgba(239,68,68,0.12)",
              border: "1px solid rgba(239,68,68,0.24)",
              color: "var(--status-error)",
            }}
          >
            {status}
          </div>
        )}

        <div className="mt-5">
          <GlassButton
            variant="primary"
            onClick={handleSubmit}
            disabled={submitting || !username.trim() || !password}
            className="w-full justify-center"
          >
            {mode === "bootstrap" ? "Create Super Admin" : "Login"}
          </GlassButton>
        </div>
      </GlassPanel>
    </div>
  );
}
