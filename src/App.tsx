import { AuthGate } from "@/src/components/auth/AuthGate";
import { AppLayout } from "@/src/components/layout/AppLayout";
import { ChatArea } from "@/src/components/chat/ChatArea";
import { SettingsModal } from "@/src/components/settings/SettingsModal";
import { useTheme } from "@/src/hooks/useTheme";

export default function App() {
  // Initialize theme detection on mount
  useTheme();

  return (
    <AuthGate>
      <AppLayout>
        <ChatArea />
        <SettingsModal />
      </AppLayout>
    </AuthGate>
  );
}
