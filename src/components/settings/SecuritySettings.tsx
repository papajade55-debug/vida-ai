import { useState } from "react";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassButton } from "@/src/design-system/GlassButton";
import { Shield, Trash2 } from "lucide-react";

export function SecuritySettings() {
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [status, setStatus] = useState<string | null>(null);

  const handleChangePassword = () => {
    if (newPassword !== confirmPassword) {
      setStatus("Passwords do not match");
      return;
    }
    if (newPassword.length < 4) {
      setStatus("Password must be at least 4 characters");
      return;
    }
    // TODO: wire to backend when PIN management is implemented
    setStatus("Password change not yet implemented");
  };

  const handleRemovePassword = () => {
    // TODO: wire to backend
    setStatus("Password removal not yet implemented");
  };

  return (
    <div className="space-y-6">
      {/* Change Password */}
      <div>
        <h3
          className="text-sm font-semibold mb-3 flex items-center gap-2"
          style={{ color: "var(--text-primary)" }}
        >
          <Shield size={16} />
          Change Password
        </h3>
        <div className="space-y-3">
          <div>
            <label
              className="block text-xs mb-1"
              style={{ color: "var(--text-secondary)" }}
            >
              Current password
            </label>
            <GlassInput
              value={currentPassword}
              onChange={setCurrentPassword}
              placeholder="Current password"
            />
          </div>
          <div>
            <label
              className="block text-xs mb-1"
              style={{ color: "var(--text-secondary)" }}
            >
              New password
            </label>
            <GlassInput
              value={newPassword}
              onChange={setNewPassword}
              placeholder="New password"
            />
          </div>
          <div>
            <label
              className="block text-xs mb-1"
              style={{ color: "var(--text-secondary)" }}
            >
              Confirm new password
            </label>
            <GlassInput
              value={confirmPassword}
              onChange={setConfirmPassword}
              placeholder="Confirm password"
            />
          </div>
          <GlassButton variant="primary" onClick={handleChangePassword}>
            Update Password
          </GlassButton>
        </div>
      </div>

      {/* Remove Password */}
      <div
        className="pt-4 border-t"
        style={{ borderColor: "var(--glass-border)" }}
      >
        <h3
          className="text-sm font-semibold mb-2 flex items-center gap-2"
          style={{ color: "var(--text-primary)" }}
        >
          <Trash2 size={16} />
          Remove Password
        </h3>
        <p className="text-xs mb-3" style={{ color: "var(--text-secondary)" }}>
          Remove password protection from the app. Anyone with access to this
          device will be able to use Vida AI.
        </p>
        <GlassButton variant="secondary" onClick={handleRemovePassword}>
          Remove Password
        </GlassButton>
      </div>

      {/* Status message */}
      {status && (
        <div
          className="text-xs px-3 py-2 rounded-[var(--radius)]"
          style={{
            background: "var(--glass-bg)",
            color: "var(--text-secondary)",
            border: "1px solid var(--glass-border)",
          }}
        >
          {status}
        </div>
      )}
    </div>
  );
}
