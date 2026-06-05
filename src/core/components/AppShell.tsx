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
  IconSettings,
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
  IconBrain,
  IconMail,
  IconClock,
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
import { ActivityLog } from "./ActivityLog";
import { NotificationBell } from "./NotificationPanel";
import { KeyboardShortcutHelp } from "./shared/KeyboardShortcutHelp";
import { useExtensionSettings } from "../extensions/useExtensionSettings";
import { extensions } from "../../extensions";

// Extension component imports
import { TemplateLibrary } from "../../extensions/templates/components/TemplateLibrary";
import { ReportList } from "../../extensions/reports/components/ReportList";
import { DocumentList } from "../../extensions/documents/components/DocumentList";
import { InvoiceList } from "../../extensions/invoices/components/InvoiceList";
import { NewsFeed } from "../../extensions/news/components/NewsFeed";
import { CalendarView } from "../../extensions/calendar/components/CalendarView";
import { AnalyticsPanel } from "../../extensions/analytics/components/AnalyticsPanel";
import { ChecklistEditor } from "../../extensions/checklists/components/ChecklistView";
import { ComplianceView } from "../../extensions/compliance/components/ComplianceView";
import { AiChat } from "../../extensions/ai/components/AiChat";

// Icon mapping for extension nav items
const extensionIcons: Record<string, React.ReactNode> = {
  IconTemplate: <IconTemplate size={16} />,
  IconReport: <IconReport size={16} />,
  IconFileText: <IconFileText size={16} />,
  IconReceipt: <IconReceipt size={16} />,
  IconNews: <IconNews size={16} />,
  IconCalendar: <IconCalendar size={16} />,
  IconChartBar: <IconChartBar size={16} />,
  IconListCheck: <IconListCheck size={16} />,
  IconShieldCheck: <IconShieldCheck size={16} />,
  IconBrain: <IconBrain size={16} />,
  IconMail: <IconMail size={16} />,
  IconClock: <IconClock size={16} />,
};

// Activity Log uses its own icon
const activityIcon = <IconClipboardList size={16} />;

type CoreView = "dashboard" | "activity" | "settings";
type ExtView = string;
type View = CoreView | ExtView;

export function AppShell() {
  const [opened, { toggle }] = useDisclosure();
  const [activeView, setActiveView] = useState<View>("dashboard");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [dashboardAction, setDashboardAction] = useState<DashboardAction | null>(null);
  const [clients, setClients] = useState<Client[]>([]);
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const { isEnabled } = useExtensionSettings();

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
      if (event.key === "?" && !event.ctrlKey && !event.metaKey && !event.altKey) {
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

  const handleDashboardActionHandled = useCallback(() => {
    setDashboardAction(null);
  }, []);

  const handleCreateClient = useCallback(() => {
    openDashboardAction({ id: Date.now(), type: "create-client" });
  }, [openDashboardAction]);

  const handleSelectClient = useCallback(
    (client: Client) => openDashboardAction({ id: Date.now(), type: "edit-client", client }),
    [openDashboardAction],
  );

  const handleCreateEngagement = useCallback(
    (clientId?: number) => openDashboardAction({ id: Date.now(), type: "create-engagement", clientId }),
    [openDashboardAction],
  );

  const handleSelectEngagement = useCallback(
    (engagement: Engagement) => openDashboardAction({ id: Date.now(), type: "edit-engagement", engagement }),
    [openDashboardAction],
  );

  const handleCreateFinding = useCallback(
    (engagementId?: number) => openDashboardAction({ id: Date.now(), type: "create-finding", engagementId }),
    [openDashboardAction],
  );

  const handleSelectFinding = useCallback(
    (finding: Finding) => openDashboardAction({ id: Date.now(), type: "edit-finding", finding }),
    [openDashboardAction],
  );

  const deliverables = extensions.filter((e) => e.category === "deliverables" && isEnabled(e.id));
  const tools = extensions.filter((e) => e.category === "tools" && isEnabled(e.id));

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
        navbar={{ width: 240, breakpoint: "sm", collapsed: { mobile: !opened } }}
        padding="md"
      >
        <MantineAppShell.Header>
          <Group h="100%" px="md" justify="space-between">
            <Group h="100%" gap="sm">
              <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
              <Title order={4}>SecuritySmith</Title>
            </Group>
            <Tooltip label="Search">
              <ActionIcon variant="light" aria-label="Search" onClick={() => setPaletteOpen(true)}>
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
          {deliverables.length > 0 && (
            <>
              <Text size="xs" c="dimmed" mt="sm" mb="xs" fw={700}>Deliverables</Text>
              {deliverables.map((ext) => (
                <NavLink
                  key={ext.id}
                  label={ext.navLabel}
                  leftSection={extensionIcons[ext.icon]}
                  active={activeView === ext.id}
                  onClick={() => setActiveView(ext.id)}
                />
              ))}
            </>
          )}
          {tools.length > 0 && (
            <>
              <Text size="xs" c="dimmed" mt="sm" mb="xs" fw={700}>Tools</Text>
              {tools.map((ext) => (
                <NavLink
                  key={ext.id}
                  label={ext.navLabel}
                  leftSection={extensionIcons[ext.icon]}
                  active={activeView === ext.id}
                  onClick={() => setActiveView(ext.id)}
                />
              ))}
            </>
          )}
          <NavLink
            label="Activity Log"
            leftSection={activityIcon}
            active={activeView === "activity"}
            onClick={() => setActiveView("activity")}
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
