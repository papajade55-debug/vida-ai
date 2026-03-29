import { AppLayout } from "@/src/components/layout/AppLayout";
import { ChatArea } from "@/src/components/chat/ChatArea";
import { useTheme } from "@/src/hooks/useTheme";

export default function App() {
  // Initialize theme detection on mount
  useTheme();

  return (
    <AppLayout>
      <ChatArea />
    </AppLayout>
  );
}
