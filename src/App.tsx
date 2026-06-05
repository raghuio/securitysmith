import { useState } from "react";
import { MantineProvider } from "@mantine/core";
import { Notifications } from "@mantine/notifications";
import { UnlockScreen } from "./core/components/UnlockScreen";
import { AppShell } from "./core/components/AppShell";
import { getBootTheme } from "./core/api/settings";

function App() {
  const [screen, setScreen] = useState<"unlock" | "app">("unlock");
  const [colorScheme, setColorScheme] = useState<"light" | "dark">("light");

  const handleUnlocked = async () => {
    try {
      const theme = await getBootTheme();
      if (theme === "light" || theme === "dark") {
        setColorScheme(theme);
      }
    } catch {
      // Keep default light on error
    }
    setScreen("app");
  };

  return (
    <MantineProvider defaultColorScheme={colorScheme}>
      <Notifications position="top-right" />
      {screen === "unlock" ? (
        <UnlockScreen onUnlocked={handleUnlocked} />
      ) : (
        <AppShell />
      )}
    </MantineProvider>
  );
}

export default App;
