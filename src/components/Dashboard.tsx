import { useState, useEffect, useCallback } from "react";
import {
  Button,
  Center,
  Group,
  Paper,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { IconPlus } from "@tabler/icons-react";
import { getDashboardStats, listClients } from "../api/clients";
import type { Client, DashboardStats as Stats } from "../api/clients";
import { listEngagements } from "../api/engagements";
import type { Engagement } from "../api/engagements";
import { TagGroup } from "./shared";
import { DashboardStats } from "./DashboardStats";
import { ClientList } from "./ClientList";
import { ClientForm } from "./ClientForm";
import { EngagementList } from "./EngagementList";
import { EngagementForm } from "./EngagementForm";

import { listFindings } from "../api/findings";
import type { Finding } from "../api/findings";
import { getActiveReminders, type Reminder } from "../api/calendar";
import { getNotifications, type Notification } from "../api/notifications";
import { FindingList } from "./FindingList";
import { FindingForm } from "./FindingForm";

type ViewMode = "dashboard" | "clients" | "engagements" | "findings";

export type DashboardAction =
  | { id: number; type: "create-client" }
  | { id: number; type: "edit-client"; client: Client }
  | { id: number; type: "create-engagement"; clientId?: number }
  | { id: number; type: "edit-engagement"; engagement: Engagement }
  | { id: number; type: "create-finding"; engagementId?: number }
  | { id: number; type: "edit-finding"; finding: Finding };

interface DashboardProps {
  clients: Client[];
  engagements: Engagement[];
  action: DashboardAction | null;
  onActionHandled: () => void;
}

export function Dashboard({
  clients,
  engagements: _engagements,
  action,
  onActionHandled,
}: DashboardProps) {
  const [stats, setStats] = useState<Stats | null>(null);
  const [recentClients, setRecentClients] = useState<Client[]>([]);
  const [recentEngagements, setRecentEngagements] = useState<Engagement[]>([]);
  const [view, setView] = useState<ViewMode>("dashboard");
  const [clientFormOpen, setClientFormOpen] = useState(false);
  const [editingClient, setEditingClient] = useState<Client | null>(null);
  const [engagementFormOpen, setEngagementFormOpen] = useState(false);
  const [editingEngagement, setEditingEngagement] = useState<Engagement | null>(
    null,
  );
  const [engagementClientId, setEngagementClientId] = useState<number | null>(
    null,
  );
  const [engagementListClientId, setEngagementListClientId] = useState<
    number | undefined
  >(undefined);
  const [refreshKey, setRefreshKey] = useState(0);
  const [recentFindings, setRecentFindings] = useState<Finding[]>([]);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [priorities, setPriorities] = useState<Notification[]>([]);
  const [findingFormOpen, setFindingFormOpen] = useState(false);
  const [editingFinding, setEditingFinding] = useState<Finding | null>(null);
  const [findingEngagementId, setFindingEngagementId] = useState<number | null>(
    null,
  );

  const refresh = useCallback(async () => {
    try {
      const s = await getDashboardStats();
      setStats(s);
      const clientsData = await listClients();
      setRecentClients(clientsData.slice(0, 5));
      const engagementsData = await listEngagements({ status: "active" });
      setRecentEngagements(engagementsData.slice(0, 5));
      const findingsData = await listFindings({ limit: 5, offset: 0 });
      setRecentFindings(findingsData.items);
      const reminderData = await getActiveReminders();
      setReminders(reminderData.slice(0, 5));
      const priorityData = await getNotifications();
      setPriorities(priorityData.slice(0, 5));
    } catch (e) {
      console.error("Failed to refresh dashboard:", e);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh, refreshKey]);

  useEffect(() => {
    if (!action) {
      return;
    }

    setView("dashboard");
    if (action.type === "create-client") {
      setEditingClient(null);
      setClientFormOpen(true);
    } else if (action.type === "edit-client") {
      setEditingClient(action.client);
      setClientFormOpen(true);
    } else if (action.type === "create-engagement") {
      setEditingEngagement(null);
      setEngagementClientId(action.clientId ?? null);
      setEngagementFormOpen(true);
    } else if (action.type === "edit-engagement") {
      setEditingEngagement(action.engagement);
      setEngagementClientId(null);
      setEngagementFormOpen(true);
    } else if (action.type === "create-finding") {
      setEditingFinding(null);
      setFindingEngagementId(action.engagementId ?? null);
      setFindingFormOpen(true);
    } else if (action.type === "edit-finding") {
      setEditingFinding(action.finding);
      setFindingEngagementId(null);
      setFindingFormOpen(true);
    }
    onActionHandled();
  }, [action, onActionHandled]);

  const handleClientSaved = () => {
    setClientFormOpen(false);
    setEditingClient(null);
    setRefreshKey((k) => k + 1);
  };

  const handleEngagementSaved = () => {
    setEngagementFormOpen(false);
    setEditingEngagement(null);
    setEngagementClientId(null);
    setRefreshKey((k) => k + 1);
  };

  const handleFindingSaved = () => {
    setFindingFormOpen(false);
    setEditingFinding(null);
    setFindingEngagementId(null);
    setRefreshKey((k) => k + 1);
  };

  const handleEditFinding = (finding: Finding) => {
    setEditingFinding(finding);
    setFindingEngagementId(null);
    setFindingFormOpen(true);
  };

  const handleEditClient = (client: Client) => {
    setEditingClient(client);
    setClientFormOpen(true);
  };

  const handleEditEngagement = (engagement: Engagement) => {
    setEditingEngagement(engagement);
    setEngagementClientId(null);
    setEngagementFormOpen(true);
  };

  const hasClients = (stats?.client_count ?? 0) > 0;
  const hasEngagements = recentEngagements.length > 0;
  const hasFindings = recentFindings.length > 0;

  if (view === "clients") {
    return (
      <>
        <ClientList
          onBack={() => setView("dashboard")}
          onEdit={handleEditClient}
          onDeleted={() => setRefreshKey((k) => k + 1)}
          onViewEngagements={(id) => {
            setEngagementListClientId(id);
            setView("engagements");
          }}
          onAddEngagement={(id) => {
            setEditingEngagement(null);
            setEngagementClientId(id);
            setEngagementFormOpen(true);
          }}
          refreshKey={refreshKey}
        />
        <ClientForm
          opened={clientFormOpen}
          client={editingClient}
          onClose={() => {
            setClientFormOpen(false);
            setEditingClient(null);
          }}
          onSaved={handleClientSaved}
        />
        <EngagementForm
          opened={engagementFormOpen}
          engagement={editingEngagement}
          clients={clients}
          preselectedClientId={engagementClientId}
          onClose={() => {
            setEngagementFormOpen(false);
            setEditingEngagement(null);
            setEngagementClientId(null);
          }}
          onSaved={handleEngagementSaved}
        />
      </>
    );
  }

  if (view === "engagements") {
    return (
      <>
        <EngagementList
          clientId={engagementListClientId}
          onBack={() => setView("dashboard")}
          onEdit={handleEditEngagement}
          onCreate={(id) => {
            setEditingEngagement(null);
            setEngagementClientId(id ?? null);
            setEngagementFormOpen(true);
          }}
          onArchived={() => setRefreshKey((k) => k + 1)}
          refreshKey={refreshKey}
        />
        <EngagementForm
          opened={engagementFormOpen}
          engagement={editingEngagement}
          clients={clients}
          preselectedClientId={engagementClientId}
          onClose={() => {
            setEngagementFormOpen(false);
            setEditingEngagement(null);
            setEngagementClientId(null);
          }}
          onSaved={handleEngagementSaved}
        />
      </>
    );
  }

  if (view === "findings") {
    return (
      <>
        <FindingList
          engagementId={findingEngagementId ?? undefined}
          onBack={() => setView("dashboard")}
          onEdit={handleEditFinding}
          onCreate={(id) => {
            setEditingFinding(null);
            setFindingEngagementId(id ?? null);
            setFindingFormOpen(true);
          }}
          onArchived={() => setRefreshKey((k) => k + 1)}
          refreshKey={refreshKey}
        />
        <FindingForm
          opened={findingFormOpen}
          finding={editingFinding}
          engagementId={findingEngagementId ?? 0}
          onClose={() => {
            setFindingFormOpen(false);
            setEditingFinding(null);
            setFindingEngagementId(null);
          }}
          onSaved={handleFindingSaved}
        />
      </>
    );
  }

  return (
    <>
      <Stack gap="lg" p="md">
        {stats && <DashboardStats stats={stats} />}

        {priorities.length > 0 && (
          <Paper withBorder p="sm" radius="md">
            <Title order={5} mb="xs">
              Today's Priorities ({priorities.length})
            </Title>
            <Stack gap="xs">
              {priorities.map((n) => (
                <Group key={n.id} gap="xs">
                  <Text size="sm" fw={600}>
                    {n.category === "deadline"
                      ? "⏰"
                      : n.category === "overdue_invoice"
                        ? "💰"
                        : n.category === "retest_due"
                          ? "🔁"
                          : n.category === "news_alert"
                            ? "📰"
                            : n.category === "follow_up"
                              ? "📧"
                              : "🔔"}
                  </Text>
                  <Text size="sm">{n.title}</Text>
                </Group>
              ))}
            </Stack>
          </Paper>
        )}

        {reminders.length > 0 && (
          <Paper withBorder p="sm" radius="md">
            <Title order={5} mb="xs">
              Upcoming ({reminders.length})
            </Title>
            <Stack gap="xs">
              {reminders.map((r) => (
                <Group key={r.reminder_key} gap="xs">
                  <Text size="sm" fw={600}>
                    {r.reminder_type === "engagement_start"
                      ? "▶"
                      : r.reminder_type === "engagement_end"
                        ? "⏹"
                        : "⚠"}
                  </Text>
                  <Text size="sm">
                    {r.entity_name} — {r.due_date}
                    {r.days_until <= 0 ? " (today)" : ` (${r.days_until}d)`}
                  </Text>
                </Group>
              ))}
            </Stack>
          </Paper>
        )}

        <Group justify="space-between" align="center">
          <Title order={4}>Recent Clients</Title>
          <Group gap="sm">
            {hasClients && (
              <Button variant="light" onClick={() => setView("clients")}>
                View all
              </Button>
            )}
            <Button
              leftSection={<IconPlus size={16} />}
              onClick={() => {
                setEditingClient(null);
                setClientFormOpen(true);
              }}
            >
              Add client
            </Button>
          </Group>
        </Group>

        {hasClients ? (
          <Stack gap="xs">
            {recentClients.map((client) => (
              <Paper
                key={client.id}
                withBorder
                shadow="xs"
                p="sm"
                radius="md"
                style={{ cursor: "pointer" }}
                onClick={() => handleEditClient(client)}
              >
                <Group justify="space-between">
                  <div>
                    <Text fw={600}>{client.name}</Text>
                    {client.contact_email && (
                      <Text size="sm" c="dimmed">
                        {client.contact_email}
                      </Text>
                    )}
                  </div>
                  {client.tags.length > 0 && (
                    <TagGroup tags={client.tags} size="xs" />
                  )}
                </Group>
              </Paper>
            ))}
          </Stack>
        ) : (
          <Center py="xl">
            <Stack align="center" gap="sm">
              <Text c="dimmed" size="lg">
                No clients yet.
              </Text>
              <Text c="dimmed" size="sm">
                Start by adding your first client.
              </Text>
              <Button
                leftSection={<IconPlus size={16} />}
                onClick={() => {
                  setEditingClient(null);
                  setClientFormOpen(true);
                }}
              >
                Add your first client
              </Button>
            </Stack>
          </Center>
        )}

        {hasClients && (
          <>
            <Group justify="space-between" align="center" mt="lg">
              <Title order={4}>Active Engagements</Title>
              <Group gap="sm">
                {hasEngagements && (
                  <Button
                    variant="light"
                    onClick={() => {
                      setEngagementListClientId(undefined);
                      setView("engagements");
                    }}
                  >
                    View all
                  </Button>
                )}
                <Button
                  leftSection={<IconPlus size={16} />}
                  onClick={() => {
                    setEditingEngagement(null);
                    setEngagementClientId(null);
                    setEngagementFormOpen(true);
                  }}
                >
                  Add engagement
                </Button>
              </Group>
            </Group>

            {hasEngagements ? (
              <Stack gap="xs">
                {recentEngagements.map((e) => (
                  <Paper
                    key={e.id}
                    withBorder
                    shadow="xs"
                    p="sm"
                    radius="md"
                    style={{ cursor: "pointer" }}
                    onClick={() => handleEditEngagement(e)}
                  >
                    <Group justify="space-between">
                      <div>
                        <Text fw={600}>{e.name}</Text>
                        <Text size="sm" c="dimmed">
                          {e.client_name} · {e.engagement_type}
                        </Text>
                        {(e.start_date || e.end_date) && (
                          <Text size="xs" c="dimmed">
                            {e.start_date ?? "Open start"} →{" "}
                            {e.end_date ?? "Open end"}
                          </Text>
                        )}
                      </div>
                      {e.tags.length > 0 && (
                        <TagGroup tags={e.tags} size="xs" />
                      )}
                    </Group>
                  </Paper>
                ))}
              </Stack>
            ) : (
              <Center py="lg">
                <Text c="dimmed" size="sm">
                  No active engagements yet. Add one from a client.
                </Text>
              </Center>
            )}
          </>
        )}

        {hasEngagements && (
          <>
            <Group justify="space-between" align="center" mt="lg">
              <Title order={4}>Recent Findings</Title>
              <Group gap="sm">
                {hasFindings && (
                  <Button
                    variant="light"
                    onClick={() => {
                      setFindingEngagementId(
                        undefined as unknown as number | null,
                      );
                      setView("findings");
                    }}
                  >
                    View all
                  </Button>
                )}
                <Button
                  leftSection={<IconPlus size={16} />}
                  onClick={() => {
                    setEditingFinding(null);
                    setFindingEngagementId(null);
                    setFindingFormOpen(true);
                  }}
                >
                  Add finding
                </Button>
              </Group>
            </Group>

            {hasFindings ? (
              <Stack gap="xs">
                {recentFindings.map((f) => (
                  <Paper
                    key={f.id}
                    withBorder
                    shadow="xs"
                    p="sm"
                    radius="md"
                    style={{ cursor: "pointer" }}
                    onClick={() => handleEditFinding(f)}
                  >
                    <Group justify="space-between">
                      <div>
                        <Group gap="sm">
                          <Text fw={600}>{f.title}</Text>
                        </Group>
                        <Text size="sm" c="dimmed">
                          {f.client_name} · {f.engagement_name}
                        </Text>
                      </div>
                      {f.tags.length > 0 && (
                        <TagGroup tags={f.tags} size="xs" />
                      )}
                    </Group>
                  </Paper>
                ))}
              </Stack>
            ) : (
              <Center py="lg">
                <Text c="dimmed" size="sm">
                  No findings yet. Add one from an engagement.
                </Text>
              </Center>
            )}
          </>
        )}
      </Stack>

      <ClientForm
        opened={clientFormOpen}
        client={editingClient}
        onClose={() => {
          setClientFormOpen(false);
          setEditingClient(null);
        }}
        onSaved={handleClientSaved}
      />

      <EngagementForm
        opened={engagementFormOpen}
        engagement={editingEngagement}
        clients={clients}
        preselectedClientId={engagementClientId}
        onClose={() => {
          setEngagementFormOpen(false);
          setEditingEngagement(null);
          setEngagementClientId(null);
        }}
        onSaved={handleEngagementSaved}
      />

      <FindingForm
        opened={findingFormOpen}
        finding={editingFinding}
        engagementId={findingEngagementId ?? 0}
        onClose={() => {
          setFindingFormOpen(false);
          setEditingFinding(null);
          setFindingEngagementId(null);
        }}
        onSaved={handleFindingSaved}
      />
    </>
  );
}
