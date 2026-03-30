import { StatusDot } from "@/src/design-system/StatusDot";
import type { ProviderInfo } from "@/src/lib/tauri";

interface AgentItemProps {
  provider: ProviderInfo;
  healthy?: boolean;
  streaming: boolean;
}

export function AgentItem({ provider, healthy, streaming }: AgentItemProps) {
  let status: "streaming" | "idle" | "offline" | "unknown";
  if (streaming) {
    status = "streaming";
  } else if (healthy === undefined) {
    status = "unknown";
  } else if (healthy) {
    status = "idle";
  } else {
    status = "offline";
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5">
      <StatusDot status={status} />
      <span className="text-sm truncate" style={{ color: "var(--text-primary)" }}>
        {provider.display_name}
      </span>
      <span className="text-xs ml-auto" style={{ color: "var(--text-secondary)" }}>
        {provider.provider_type}
      </span>
    </div>
  );
}
