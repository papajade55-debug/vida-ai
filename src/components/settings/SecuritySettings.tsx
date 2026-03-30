import { useMemo, useState } from "react";
import { Shield, Trash2, UserPlus, LogOut } from "lucide-react";
import { GlassInput } from "@/src/design-system/GlassInput";
import { GlassButton } from "@/src/design-system/GlassButton";
import { useAuth } from "@/src/hooks/useAuth";
import type { ActorRole } from "@/src/lib/tauri";

const ROLE_OPTIONS: { value: ActorRole; label: string }[] = [
  { value: "architect", label: "Architecte" },
  { value: "operator", label: "Operateur" },
];

export function SecuritySettings() {
  const { authActor, authUsers, createUser, changePassword, logout } = useAuth();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [newUsername, setNewUsername] = useState("");
  const [newUserPassword, setNewUserPassword] = useState("");
  const [newUserRole, setNewUserRole] = useState<ActorRole>("architect");
  const [status, setStatus] = useState<string | null>(null);

  const canManageUsers = useMemo(
    () => authActor?.role === "super_admin",
    [authActor],
  );

  const handleChangePassword = async () => {
    try {
      if (newPassword !== confirmPassword) {
        throw new Error("Passwords do not match");
      }
      await changePassword(currentPassword, newPassword);
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      setStatus("Password updated");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  };

  const handleCreateUser = async () => {
    try {
      await createUser(newUsername.trim(), newUserPassword, newUserRole);
      setNewUsername("");
      setNewUserPassword("");
      setStatus("User created");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  };

  const handleLogout = async () => {
    await logout();
  };

  return (
    <div className="space-y-6">
      <div>
        <h3
          className="text-sm font-semibold mb-3 flex items-center gap-2"
          style={{ color: "var(--text-primary)" }}
        >
          <Shield size={16} />
          Active Session
        </h3>
        <div
          className="rounded-[var(--radius)] px-3 py-3 text-sm"
          style={{
            background: "var(--glass-bg)",
            border: "1px solid var(--glass-border)",
            color: "var(--text-primary)",
          }}
        >
          <div>{authActor?.username ?? "Not authenticated"}</div>
          <div className="text-xs mt-1" style={{ color: "var(--text-secondary)" }}>
            Role: {authActor?.role ?? "none"}
          </div>
        </div>
        <div className="mt-3">
          <GlassButton variant="secondary" onClick={handleLogout} icon={<LogOut size={14} />}>
            Logout
          </GlassButton>
        </div>
      </div>

      <div>
        <h3
          className="text-sm font-semibold mb-3 flex items-center gap-2"
          style={{ color: "var(--text-primary)" }}
        >
          <Shield size={16} />
          Change Password
        </h3>
        <div className="space-y-3">
          <GlassInput
            value={currentPassword}
            onChange={setCurrentPassword}
            placeholder="Current password"
          />
          <GlassInput
            value={newPassword}
            onChange={setNewPassword}
            placeholder="New password"
          />
          <GlassInput
            value={confirmPassword}
            onChange={setConfirmPassword}
            placeholder="Confirm new password"
          />
          <GlassButton variant="primary" onClick={handleChangePassword}>
            Update Password
          </GlassButton>
        </div>
      </div>

      {canManageUsers && (
        <div className="pt-4 border-t" style={{ borderColor: "var(--glass-border)" }}>
          <h3
            className="text-sm font-semibold mb-3 flex items-center gap-2"
            style={{ color: "var(--text-primary)" }}
          >
            <UserPlus size={16} />
            User Management
          </h3>
          <div className="space-y-3">
            <GlassInput
              value={newUsername}
              onChange={setNewUsername}
              placeholder="New username"
            />
            <GlassInput
              value={newUserPassword}
              onChange={setNewUserPassword}
              placeholder="Temporary password"
            />
            <select
              value={newUserRole}
              onChange={(event) => setNewUserRole(event.target.value as ActorRole)}
              className="w-full px-3 py-2 rounded-[var(--radius)] outline-none"
              style={{
                background: "var(--glass-bg)",
                color: "var(--text-primary)",
                border: "1px solid var(--glass-border)",
              }}
            >
              {ROLE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            <GlassButton variant="primary" onClick={handleCreateUser}>
              Create User
            </GlassButton>
          </div>

          <div className="mt-4 space-y-2">
            {authUsers.map((user) => (
              <div
                key={user.id}
                className="rounded-[var(--radius)] px-3 py-2 text-sm"
                style={{
                  background: "var(--glass-bg)",
                  border: "1px solid var(--glass-border)",
                  color: "var(--text-primary)",
                }}
              >
                <div>{user.username}</div>
                <div className="text-xs mt-1" style={{ color: "var(--text-secondary)" }}>
                  {user.role}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {!canManageUsers && (
        <div
          className="text-xs px-3 py-2 rounded-[var(--radius)]"
          style={{
            background: "var(--glass-bg)",
            color: "var(--text-secondary)",
            border: "1px solid var(--glass-border)",
          }}
        >
          Team and user administration is restricted to Super Admin.
        </div>
      )}

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
