import { useEffect, useState } from "react";
import {
  Button,
  Group,
  Paper,
  Badge,
  Stack,
  Text,
  Title,
  UnstyledButton,
} from "@mantine/core";
import { IconChevronLeft, IconChevronRight } from "@tabler/icons-react";
import { listCalendarEvents, type CalendarEvent } from "../api/calendar";

function formatIso(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function startOfWeek(date: Date): Date {
  const d = new Date(date);
  const day = d.getDay();
  d.setDate(d.getDate() - day);
  d.setHours(0, 0, 0, 0);
  return d;
}

function addDays(date: Date, days: number): Date {
  const d = new Date(date);
  d.setDate(d.getDate() + days);
  return d;
}

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function CalendarView() {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [viewMode, setViewMode] = useState<"month" | "week">("month");
  const [currentYear, setCurrentYear] = useState(new Date().getFullYear());
  const [currentMonth, setCurrentMonth] = useState(new Date().getMonth());
  const [weekStart, setWeekStart] = useState(startOfWeek(new Date()));
  const [selectedDate, setSelectedDate] = useState<Date>(new Date());

  const load = async () => {
    try {
      const data = await listCalendarEvents();
      setEvents(data);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const getDayEvents = (date: Date) => {
    const dateStr = formatIso(date);
    return events.filter((e) => {
      const start = e.start_date || "";
      const end = e.end_date || "";
      return dateStr >= start && dateStr <= end;
    });
  };

  const isSameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();

  // Month navigation
  const prevMonth = () => {
    if (currentMonth === 0) {
      setCurrentMonth(11);
      setCurrentYear((y) => y - 1);
    } else {
      setCurrentMonth((m) => m - 1);
    }
  };

  const nextMonth = () => {
    if (currentMonth === 11) {
      setCurrentMonth(0);
      setCurrentYear((y) => y + 1);
    } else {
      setCurrentMonth((m) => m + 1);
    }
  };

  // Week navigation
  const prevWeek = () => setWeekStart((w) => addDays(w, -7));
  const nextWeek = () => setWeekStart((w) => addDays(w, 7));

  const monthLabel = new Date(currentYear, currentMonth).toLocaleString(
    undefined,
    { month: "long", year: "numeric" },
  );

  const weekLabel = `${weekStart.toLocaleDateString(undefined, { month: "short", day: "numeric" })} – ${addDays(weekStart, 6).toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;

  const selectedDayEvents = getDayEvents(selectedDate);

  const weekDays = Array.from({ length: 7 }, (_, i) => addDays(weekStart, i));

  return (
    <Stack gap="md" p="md">
      <Group justify="space-between">
        <Title order={3}>Calendar</Title>
        <Group gap="xs">
          <Button
            variant={viewMode === "month" ? "filled" : "light"}
            size="xs"
            onClick={() => setViewMode("month")}
          >
            Month
          </Button>
          <Button
            variant={viewMode === "week" ? "filled" : "light"}
            size="xs"
            onClick={() => setViewMode("week")}
          >
            Week
          </Button>
          <Button size="xs" onClick={load}>
            Refresh
          </Button>
        </Group>
      </Group>

      <Group align="flex-start" gap="md">
        <Paper withBorder p="sm" radius="md" style={{ minWidth: 320 }}>
          <Group justify="space-between" mb="sm">
            <UnstyledButton
              onClick={viewMode === "month" ? prevMonth : prevWeek}
            >
              <IconChevronLeft size={20} />
            </UnstyledButton>
            <Text fw={600}>
              {viewMode === "month" ? monthLabel : weekLabel}
            </Text>
            <UnstyledButton
              onClick={viewMode === "month" ? nextMonth : nextWeek}
            >
              <IconChevronRight size={20} />
            </UnstyledButton>
          </Group>

          {viewMode === "month" ? (
            <MonthGrid
              year={currentYear}
              month={currentMonth}
              selectedDate={selectedDate}
              onSelectDate={setSelectedDate}
              getDayEvents={getDayEvents}
              isSameDay={isSameDay}
            />
          ) : (
            <WeekGrid
              days={weekDays}
              selectedDate={selectedDate}
              onSelectDate={setSelectedDate}
              getDayEvents={getDayEvents}
              isSameDay={isSameDay}
            />
          )}
        </Paper>

        <Stack gap="sm" style={{ flex: 1 }}>
          <Text fw={600}>
            {selectedDate.toLocaleDateString(undefined, {
              weekday: "long",
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </Text>
          {selectedDayEvents.length === 0 && (
            <Text c="dimmed" size="sm">
              No engagements on this day.
            </Text>
          )}
          {selectedDayEvents.map((e) => (
            <Paper key={e.id} withBorder p="sm" radius="sm">
              <Group justify="space-between">
                <Text fw={500}>{e.name}</Text>
                <Badge size="xs" variant="light">
                  {e.status}
                </Badge>
              </Group>
              <Text size="xs" c="dimmed">
                {e.client_name} · {e.start_date || "—"} → {e.end_date || "—"}
              </Text>
            </Paper>
          ))}
        </Stack>
      </Group>
    </Stack>
  );
}

function MonthGrid({
  year,
  month,
  selectedDate,
  onSelectDate,
  getDayEvents,
  isSameDay,
}: {
  year: number;
  month: number;
  selectedDate: Date;
  onSelectDate: (d: Date) => void;
  getDayEvents: (d: Date) => CalendarEvent[];
  isSameDay: (a: Date, b: Date) => boolean;
}) {
  const lastDay = new Date(year, month + 1, 0);
  const days: Date[] = [];
  for (let d = 1; d <= lastDay.getDate(); d++) {
    days.push(new Date(year, month, d));
  }
  const firstWeekday = days[0]?.getDay() || 0;

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(7, 1fr)",
        gap: 4,
      }}
    >
      {WEEKDAYS.map((wd) => (
        <Text key={wd} size="xs" c="dimmed" ta="center">
          {wd}
        </Text>
      ))}
      {Array.from({ length: firstWeekday }).map((_, i) => (
        <div key={`pad-${i}`} />
      ))}
      {days.map((day) => {
        const dayEvents = getDayEvents(day);
        const active = isSameDay(day, selectedDate);
        return (
          <UnstyledButton
            key={day.getDate()}
            onClick={() => onSelectDate(day)}
            style={{
              width: "100%",
              padding: "8px 0",
              borderRadius: 4,
              background: active
                ? "var(--mantine-color-blue-light)"
                : "transparent",
              position: "relative",
            }}
          >
            <Text size="sm" ta="center" fw={active ? 700 : 400}>
              {day.getDate()}
            </Text>
            {dayEvents.length > 0 && (
              <span
                style={{
                  position: "absolute",
                  bottom: 2,
                  left: "50%",
                  transform: "translateX(-50%)",
                  width: 4,
                  height: 4,
                  borderRadius: "50%",
                  background: "var(--mantine-color-blue-filled)",
                }}
              />
            )}
          </UnstyledButton>
        );
      })}
    </div>
  );
}

function WeekGrid({
  days,
  selectedDate,
  onSelectDate,
  getDayEvents,
  isSameDay,
}: {
  days: Date[];
  selectedDate: Date;
  onSelectDate: (d: Date) => void;
  getDayEvents: (d: Date) => CalendarEvent[];
  isSameDay: (a: Date, b: Date) => boolean;
}) {
  return (
    <Stack gap="xs">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(7, 1fr)",
          gap: 4,
        }}
      >
        {days.map((day) => {
          const active = isSameDay(day, selectedDate);
          return (
            <UnstyledButton
              key={day.toISOString()}
              onClick={() => onSelectDate(day)}
              style={{
                padding: "8px 4px",
                borderRadius: 4,
                background: active
                  ? "var(--mantine-color-blue-light)"
                  : "transparent",
              }}
            >
              <Text size="xs" c="dimmed" ta="center">
                {WEEKDAYS[day.getDay()]}
              </Text>
              <Text size="sm" ta="center" fw={active ? 700 : 400}>
                {day.getDate()}
              </Text>
            </UnstyledButton>
          );
        })}
      </div>
      <Stack gap="xs">
        {days.map((day) => {
          const dayEvents = getDayEvents(day);
          if (dayEvents.length === 0) return null;
          const active = isSameDay(day, selectedDate);
          return (
            <Paper
              key={day.toISOString()}
              withBorder
              p="xs"
              radius="sm"
              style={{
                borderColor: active
                  ? "var(--mantine-color-blue-filled)"
                  : undefined,
              }}
              onClick={() => onSelectDate(day)}
            >
              <Text size="xs" fw={600} mb={4}>
                {day.toLocaleDateString(undefined, {
                  weekday: "short",
                  month: "short",
                  day: "numeric",
                })}
              </Text>
              {dayEvents.map((e) => (
                <Text key={e.id} size="xs" c="dimmed">
                  {e.name} ({e.status})
                </Text>
              ))}
            </Paper>
          );
        })}
      </Stack>
    </Stack>
  );
}
