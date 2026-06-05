import { useEffect, useRef, useState } from "react";
import {
  Button,
  Card,
  Group,
  Progress,
  Stack,
  Text,
  Title,
  Badge,
} from "@mantine/core";
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerStop,
} from "@tabler/icons-react";
import {
  listTimeEntries,
  createTimeEntry,
  getBudgetStatus,
  type TimeEntry,
  type BudgetStatus,
} from "../../time-tracking/api";
import { TimeEntryForm } from "./TimeEntryForm";

export function TimeTracker({ engagementId }: { engagementId: number }) {
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [budget, setBudget] = useState<BudgetStatus | null>(null);
  const [running, setRunning] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [showForm, setShowForm] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const load = async () => {
    try {
      const [ent, bud] = await Promise.all([
        listTimeEntries(engagementId),
        getBudgetStatus(),
      ]);
      setEntries(ent);
      setBudget(bud.find((b) => b.engagement_id === engagementId) || null);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [engagementId]);

  const startTimer = () => {
    setRunning(true);
    timerRef.current = setInterval(() => {
      setElapsed((prev) => prev + 1);
    }, 1000);
  };

  const pauseTimer = () => {
    setRunning(false);
    if (timerRef.current) clearInterval(timerRef.current);
  };

  const stopTimer = () => {
    pauseTimer();
    setShowForm(true);
  };

  const handleSaveEntry = async (input: {
    entry_date: string;
    hours: number;
    description?: string;
    activity_type: string;
    is_billable?: boolean;
  }) => {
    try {
      await createTimeEntry({
        engagement_id: engagementId,
        ...input,
      });
      setElapsed(0);
      setShowForm(false);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const formatTime = (seconds: number) => {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  };

  const totalHours = entries.reduce((sum, e) => sum + e.hours, 0);

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={5}>Time Tracking</Title>
        {budget && (
          <Badge
            size="lg"
            color={
              budget.percentage > 100
                ? "red"
                : budget.percentage > 75
                  ? "yellow"
                  : "green"
            }
          >
            {totalHours.toFixed(1)}h / {budget.budgeted_hours.toFixed(1)}h
          </Badge>
        )}
      </Group>

      {budget && (
        <Progress
          value={Math.min(budget.percentage, 100)}
          size="sm"
          color={
            budget.percentage > 100
              ? "red"
              : budget.percentage > 75
                ? "yellow"
                : "green"
          }
        />
      )}

      <Card withBorder padding="sm" radius="md">
        <Group justify="space-between">
          <Text fw={600} size="xl" ff="monospace">
            {formatTime(elapsed)}
          </Text>
          <Group gap="xs">
            {!running ? (
              <Button
                leftSection={<IconPlayerPlay size={16} />}
                size="xs"
                onClick={startTimer}
              >
                Start
              </Button>
            ) : (
              <Button
                leftSection={<IconPlayerPause size={16} />}
                size="xs"
                variant="light"
                onClick={pauseTimer}
              >
                Pause
              </Button>
            )}
            <Button
              leftSection={<IconPlayerStop size={16} />}
              size="xs"
              color="red"
              variant="light"
              onClick={stopTimer}
            >
              Stop
            </Button>
          </Group>
        </Group>
      </Card>

      {showForm && (
        <TimeEntryForm
          prefillHours={Math.round((elapsed / 3600) * 4) / 4}
          onSave={handleSaveEntry}
          onCancel={() => {
            setShowForm(false);
            setElapsed(0);
          }}
        />
      )}

      <Stack gap="sm">
        {entries.map((e) => (
          <Card key={e.id} withBorder padding="sm" radius="md">
            <Group justify="space-between">
              <Group gap="sm">
                <Text size="sm">{e.entry_date}</Text>
                <Badge size="sm" variant="light">
                  {e.activity_type}
                </Badge>
                <Text size="sm" fw={600}>
                  {e.hours}h
                </Text>
              </Group>
              <Badge size="sm" color={e.is_billable ? "green" : "gray"}>
                {e.is_billable ? "Billable" : "Non-billable"}
              </Badge>
            </Group>
            {e.description && (
              <Text size="xs" c="dimmed">
                {e.description}
              </Text>
            )}
          </Card>
        ))}
        {entries.length === 0 && (
          <Text c="dimmed" size="sm">
            No time entries
          </Text>
        )}
      </Stack>
    </Stack>
  );
}
