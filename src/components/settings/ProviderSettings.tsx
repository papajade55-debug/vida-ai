import { useState, useEffect } from "react";
import { RefreshCw, Plus, Trash2 } from "lucide-react";
import { GlassButton } from "@/src/design-system/GlassButton";
import { GlassInput } from "@/src/design-system/GlassInput";
import { StatusDot } from "@/src/design-system/StatusDot";
import { useProviders } from "@/src/hooks/useProviders";
import { api } from "@/src/lib/tauri";

export function ProviderSettings() {
  const { providers, health, refresh } = useProviders();
  const [refreshing, setRefreshing] = useState(false);
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [saveStatus, setSaveStatus] = useState<Record<string, string>>({});

  const handleRefresh = async () => {
    setRefreshing(true);
    await refresh();
    setRefreshing(false);
  };

  const handleSaveKey = async (providerId: string) => {
    if (!apiKeyInput.trim()) return;
    try {
      await api.storeApiKey(providerId, apiKeyInput.trim());
      setSaveStatus((prev) => ({ ...prev, [providerId]: "Saved" }));
      setApiKeyInput("");
      setEditingProvider(null);
      // Refresh health after saving key
      setTimeout(() => refresh(), 500);
    } catch (err) {
      setSaveStatus((prev) => ({
        ...prev,
        [providerId]: `Error: ${err}`,
      }));
    }
  };

  const handleRemoveKey = async (providerId: string) => {
    try {
      await api.removeApiKey(providerId);
      setSaveStatus((prev) => ({ ...prev, [providerId]: "Key removed" }));
      setTimeout(() => refresh(), 500);
    } catch (err) {
      setSaveStatus((prev) => ({
        ...prev,
        [providerId]: `Error: ${err}`,
      }));
    }
  };

  return (
    <div className="space-y-4">
      {/* Header with refresh */}
      <div className="flex items-center justify-between">
        <span
          className="text-sm font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          Configured Providers
        </span>
        <GlassButton
          variant="ghost"
          icon={
            <RefreshCw
              size={14}
              className={refreshing ? "animate-spin" : ""}
            />
          }
          onClick={handleRefresh}
          title="Refresh"
          className="!px-2 !py-1"
        />
      </div>

      {/* Provider list */}
      {providers.length === 0 ? (
        <div
          className="text-sm text-center py-4"
          style={{ color: "var(--text-secondary)" }}
        >
          No providers configured
        </div>
      ) : (
        <div className="space-y-2">
          {providers.map((provider) => {
            const isHealthy = health[provider.name] ?? false;
            const isEditing = editingProvider === provider.name;

            return (
              <div
                key={provider.name}
                className="px-3 py-2 rounded-[var(--radius)]"
                style={{
                  background: "var(--glass-bg)",
                  border: "1px solid var(--glass-border)",
                }}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <StatusDot status={isHealthy ? "idle" : "offline"} />
                    <span
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {provider.name}
                    </span>
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded"
                      style={{
                        background: "var(--glass-border)",
                        color: "var(--text-secondary)",
                      }}
                    >
                      {provider.provider_type}
                    </span>
                  </div>
                  <div className="flex gap-1">
                    <GlassButton
                      variant="ghost"
                      icon={<Plus size={14} />}
                      onClick={() => {
                        setEditingProvider(
                          isEditing ? null : provider.name
                        );
                        setApiKeyInput("");
                      }}
                      title="Set API Key"
                      className="!px-1.5 !py-1"
                    />
                    <GlassButton
                      variant="ghost"
                      icon={<Trash2 size={14} />}
                      onClick={() => handleRemoveKey(provider.name)}
                      title="Remove API Key"
                      className="!px-1.5 !py-1"
                    />
                  </div>
                </div>

                {/* API key editor */}
                {isEditing && (
                  <div className="mt-2 flex gap-2">
                    <div className="flex-1">
                      <GlassInput
                        value={apiKeyInput}
                        onChange={setApiKeyInput}
                        placeholder="Enter API key..."
                      />
                    </div>
                    <GlassButton
                      variant="primary"
                      onClick={() => handleSaveKey(provider.name)}
                      disabled={!apiKeyInput.trim()}
                    >
                      Save
                    </GlassButton>
                  </div>
                )}

                {/* Status message */}
                {saveStatus[provider.name] && (
                  <div
                    className="text-[10px] mt-1"
                    style={{ color: "var(--text-secondary)" }}
                  >
                    {saveStatus[provider.name]}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
