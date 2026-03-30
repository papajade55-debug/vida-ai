import { useProviders } from "@/src/hooks/useProviders";
import { useStore } from "@/src/stores/store";
import { AgentItem } from "./AgentItem";

export function AgentList() {
  const { providers, health } = useProviders();
  const streamingMessageId = useStore((s) => s.streamingMessageId);

  if (providers.length === 0) {
    return (
      <div className="px-3 py-2 text-xs" style={{ color: "var(--text-secondary)" }}>
        No providers configured
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {providers.map((provider) => (
        <AgentItem
          key={provider.id}
          provider={provider}
          healthy={health[provider.id] ?? false}
          streaming={streamingMessageId !== null}
        />
      ))}
    </div>
  );
}
