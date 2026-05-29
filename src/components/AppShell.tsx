import { useState } from "react";
import {
  AppShell as MantineAppShell,
  Burger,
  Group,
  NavLink,
  Title,
} from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { Dashboard } from "./Dashboard";
import { SettingsPage } from "./SettingsPage";

type View = "dashboard" | "settings";

export function AppShell() {
  const [opened, { toggle }] = useDisclosure();
  const [activeView, setActiveView] = useState<View>("dashboard");

  return (
    <MantineAppShell
      header={{ height: 60 }}
      navbar={{
        width: 240,
        breakpoint: "sm",
        collapsed: { mobile: !opened },
      }}
      padding="md"
    >
      <MantineAppShell.Header>
        <Group h="100%" px="md">
          <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
          <Title order={4}>SecuritySmith</Title>
        </Group>
      </MantineAppShell.Header>

      <MantineAppShell.Navbar p="md">
        <NavLink
          label="Dashboard"
          active={activeView === "dashboard"}
          onClick={() => setActiveView("dashboard")}
        />
        <NavLink
          label="Settings"
          active={activeView === "settings"}
          onClick={() => setActiveView("settings")}
        />
      </MantineAppShell.Navbar>

      <MantineAppShell.Main>
        {activeView === "dashboard" ? <Dashboard /> : <SettingsPage />}
      </MantineAppShell.Main>
    </MantineAppShell>
  );
}
