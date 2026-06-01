import { useCallback, useEffect, useState } from "react";
import {
  ActionIcon,
  AppShell as MantineAppShell,
  Burger,
  Group,
  NavLink,
  Text,
  Title,
  Tooltip,
} from "@mantine/core";
import {
  IconSearch,
  IconDashboard,
  IconTemplate,
  IconReport,
  IconFileText,
  IconReceipt,
  IconNews,
  IconCalendar,
  IconClipboardList,
  IconChartBar,
  IconListCheck,
  IconShieldCheck,
  IconSettings,
} from "@tabler/icons-react";
import type { Client } from "../api/clients";
import type { Engagement } from "../api/engagements";
import type { Finding } from "../api/findings";
import { listClients } from "../api/clients";
import { listEngagements } from "../api/engagements";
import { CommandPalette } from "./CommandPalette";
import { useDisclosure } from "@mantine/hooks";
import { Dashboard } from "./Dashboard";
import type { DashboardAction } from "./Dashboard";
import { SettingsPage } from "./SettingsPage";
import { TemplateLibrary } from "./TemplateLibrary";
import { ReportList } from "./ReportList";
import { DocumentList } from "./DocumentList";
import { InvoiceList } from "./InvoiceList";
import { NewsFeed } from "./NewsFeed";
import { CalendarView } from "./CalendarView";
import { ActivityLog } from "./ActivityLog";
import { AiChat } from "./AiChat";
import { NotificationBell } from "./NotificationPanel";
import { AnalyticsPanel } from "./AnalyticsPanel";
import { ChecklistEditor } from "./ChecklistView";
import { ComplianceView } from "./ComplianceView";
import { KeyboardShortcutHelp } from "./shared/KeyboardShortcutHelp";

type View =
  | "dashboard"
  | "templates"
  | "reports"
  | "documents"
  | "invoices"
  | "news"
  | "calendar"
  | "activity"
  | "analytics"
  | "checklists"
  | "compliance"
  | "settings";

export function AppShell() {
  const [opened, { toggle }] = useDisclosure();
  const [activeView, setActiveView] = useState<View>("dashboard");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [dashboardAction, setDashboardAction] =
    useState<DashboardAction | null>(null);
  const [clients, setClients] = useState<Client[]>([]);
  const [engagements, setEngagements] = useState<Engagement[]>([]);

  useEffect(() => {
    const load = async () => {
      try {
        const [clientData, engagementData] = await Promise.all([
          listClients(),
          listEngagements(),
        ]);
        setClients(clientData);
        setEngagements(engagementData);
      } catch (e) {
        console.error("Failed to load shell data:", e);
      }
    };
    if (activeView === "dashboard") {
      load();
    }
  }, [activeView]);

  const [shortcutHelpOpen, setShortcutHelpOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen(true);
      }
      if (
        event.key === "?" &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      ) {
        setShortcutHelpOpen(true);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const openDashboardAction = useCallback((action: DashboardAction) => {
    setActiveView("dashboard");
    setDashboardAction(action);
  }, []);

  const handleCreateClient = useCallback(() => {
    openDashboardAction({ id: Date.now(), type: "create-client" });
  }, [openDashboardAction]);

  const handleSelectClient = useCallback(
    (client: Client) => {
      openDashboardAction({ id: Date.now(), type: "edit-client", client });
    },
    [openDashboardAction],
  );

  const handleCreateEngagement = useCallback(
    (clientId?: number) => {
      openDashboardAction({
        id: Date.now(),
        type: "create-engagement",
        clientId,
      });
    },
    [openDashboardAction],
  );

  const handleSelectEngagement = useCallback(
    (engagement: Engagement) => {
      openDashboardAction({
        id: Date.now(),
        type: "edit-engagement",
        engagement,
      });
    },
    [openDashboardAction],
  );

  const handleCreateFinding = useCallback(
    (engagementId?: number) => {
      openDashboardAction({
        id: Date.now(),
        type: "create-finding",
        engagementId,
      });
    },
    [openDashboardAction],
  );

  const handleSelectFinding = useCallback(
    (finding: Finding) => {
      openDashboardAction({
        id: Date.now(),
        type: "edit-finding",
        finding,
      });
    },
    [openDashboardAction],
  );

  const handleDashboardActionHandled = useCallback(() => {
    setDashboardAction(null);
  }, []);

  return (
    <>
      <CommandPalette
        opened={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onCreateClient={handleCreateClient}
        onSelectClient={handleSelectClient}
        onCreateEngagement={handleCreateEngagement}
        onSelectEngagement={handleSelectEngagement}
        onCreateFinding={handleCreateFinding}
        onSelectFinding={handleSelectFinding}
      />

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
          <Group h="100%" px="md" justify="space-between">
            <Group h="100%" gap="sm">
              <Burger
                opened={opened}
                onClick={toggle}
                hiddenFrom="sm"
                size="sm"
              />
              <Title order={4}>SecuritySmith</Title>
            </Group>
            <Tooltip label="Search">
              <ActionIcon
                variant="light"
                aria-label="Search"
                onClick={() => setPaletteOpen(true)}
              >
                <IconSearch size={18} />
              </ActionIcon>
            </Tooltip>
            <NotificationBell />
          </Group>
        </MantineAppShell.Header>

        <MantineAppShell.Navbar p="md">
          <NavLink
            label="Dashboard"
            leftSection={<IconDashboard size={16} />}
            active={activeView === "dashboard"}
            onClick={() => setActiveView("dashboard")}
          />
          <Text size="xs" c="dimmed" mt="sm" mb="xs" fw={700}>
            Deliverables
          </Text>
          <NavLink
            label="Templates"
            leftSection={<IconTemplate size={16} />}
            active={activeView === "templates"}
            onClick={() => setActiveView("templates")}
          />
          <NavLink
            label="Reports"
            leftSection={<IconReport size={16} />}
            active={activeView === "reports"}
            onClick={() => setActiveView("reports")}
          />
          <NavLink
            label="Documents"
            leftSection={<IconFileText size={16} />}
            active={activeView === "documents"}
            onClick={() => setActiveView("documents")}
          />
          <NavLink
            label="Invoices"
            leftSection={<IconReceipt size={16} />}
            active={activeView === "invoices"}
            onClick={() => setActiveView("invoices")}
          />
          <Text size="xs" c="dimmed" mt="sm" mb="xs" fw={700}>
            Tools
          </Text>
          <NavLink
            label="News"
            leftSection={<IconNews size={16} />}
            active={activeView === "news"}
            onClick={() => setActiveView("news")}
          />
          <NavLink
            label="Calendar"
            leftSection={<IconCalendar size={16} />}
            active={activeView === "calendar"}
            onClick={() => setActiveView("calendar")}
          />
          <NavLink
            label="Activity Log"
            leftSection={<IconClipboardList size={16} />}
            active={activeView === "activity"}
            onClick={() => setActiveView("activity")}
          />
          <NavLink
            label="Analytics"
            leftSection={<IconChartBar size={16} />}
            active={activeView === "analytics"}
            onClick={() => setActiveView("analytics")}
          />
          <NavLink
            label="Checklists"
            leftSection={<IconListCheck size={16} />}
            active={activeView === "checklists"}
            onClick={() => setActiveView("checklists")}
          />
          <NavLink
            label="Compliance"
            leftSection={<IconShieldCheck size={16} />}
            active={activeView === "compliance"}
            onClick={() => setActiveView("compliance")}
          />
          <div style={{ marginTop: "auto" }}>
            <NavLink
              label="Settings"
              leftSection={<IconSettings size={16} />}
              active={activeView === "settings"}
              onClick={() => setActiveView("settings")}
            />
          </div>
        </MantineAppShell.Navbar>

        <MantineAppShell.Main>
          {activeView === "dashboard" && (
            <Dashboard
              clients={clients}
              engagements={engagements}
              action={dashboardAction}
              onActionHandled={handleDashboardActionHandled}
            />
          )}
          {activeView === "templates" && <TemplateLibrary />}
          {activeView === "reports" && <ReportList />}
          {activeView === "documents" && <DocumentList />}
          {activeView === "invoices" && <InvoiceList />}
          {activeView === "news" && <NewsFeed />}
          {activeView === "calendar" && <CalendarView />}
          {activeView === "activity" && <ActivityLog />}
          {activeView === "analytics" && <AnalyticsPanel />}
          {activeView === "checklists" && <ChecklistEditor />}
          {activeView === "compliance" && <ComplianceView findingId={0} />}
          {activeView === "settings" && <SettingsPage />}
        </MantineAppShell.Main>
      </MantineAppShell>
      <AiChat />
      <KeyboardShortcutHelp
        opened={shortcutHelpOpen}
        onClose={() => setShortcutHelpOpen(false)}
      />
    </>
  );
}
