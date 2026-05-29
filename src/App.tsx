import { useState } from "react";
import { UnlockScreen } from "./components/UnlockScreen";
import { AppShell } from "./components/AppShell";

function App() {
  const [unlocked, setUnlocked] = useState(false);

  if (!unlocked) {
    return <UnlockScreen onUnlocked={() => setUnlocked(true)} />;
  }

  return <AppShell />;
}

export default App;
