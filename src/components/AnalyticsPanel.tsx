import { useEffect, useState } from "react";
import {
  Card,
  Group,
  Select,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import { BarChart, DonutChart, LineChart } from "@mantine/charts";
import {
  getSeverityDistribution,
  getTopCategories,
  getFindingsOverTime,
  getRemediationRate,
  getEngagementTimeline,
  getTimeByActivity,
  getBudgetVsActual,
  getRevenueByClient,
  type DataPoint,
  type TimeSeriesPoint,
  type RemediationRate,
  type BudgetComparison,
  type TimelineEntry,
} from "../api/analytics";

const RANGE_OPTIONS = [
  { value: "30d", label: "30d" },
  { value: "90d", label: "90d" },
  { value: "6m", label: "6m" },
  { value: "1y", label: "1y" },
  { value: "all", label: "All" },
];

function rangeToDates(range: string): {
  from: string | null;
  to: string | null;
} {
  if (range === "all") return { from: null, to: null };
  const days =
    range === "30d" ? 30 : range === "90d" ? 90 : range === "6m" ? 182 : 365;
  const now = new Date();
  const past = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);
  return {
    from: past.toISOString().slice(0, 10),
    to: now.toISOString().slice(0, 10),
  };
}

export function AnalyticsPanel() {
  const [severityDist, setSeverityDist] = useState<DataPoint[]>([]);
  const [topCategories, setTopCategories] = useState<DataPoint[]>([]);
  const [findingsTime, setFindingsTime] = useState<TimeSeriesPoint[]>([]);
  const [remediation, setRemediation] = useState<RemediationRate | null>(null);
  const [timeline, setTimeline] = useState<TimelineEntry[]>([]);
  const [timeActivity, setTimeActivity] = useState<DataPoint[]>([]);
  const [budget, setBudget] = useState<BudgetComparison[]>([]);
  const [revenue, setRevenue] = useState<DataPoint[]>([]);
  const [interval, setInterval] = useState("monthly");
  const [range, setRange] = useState("all");

  const load = async () => {
    try {
      const { from, to } = rangeToDates(range);
      const [sev, top, ft, rem, tl, ta, bud, rev] = await Promise.all([
        getSeverityDistribution(from ?? undefined, to ?? undefined),
        getTopCategories(10),
        getFindingsOverTime(interval),
        getRemediationRate(),
        getEngagementTimeline(),
        getTimeByActivity(),
        getBudgetVsActual(),
        getRevenueByClient(),
      ]);
      setSeverityDist(sev);
      setTopCategories(top);
      setFindingsTime(ft);
      setRemediation(rem);
      setTimeline(tl);
      setTimeActivity(ta);
      setBudget(bud);
      setRevenue(rev);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [interval, range]);

  const severityColors = ["red", "orange", "yellow", "blue", "gray"];

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Title order={3}>Analytics</Title>
        <Group gap="xs">
          <Select
            value={range}
            onChange={(v) => v && setRange(v)}
            data={RANGE_OPTIONS}
            w={100}
            aria-label="Date range"
          />
          <Select
            value={interval}
            onChange={(v) => v && setInterval(v)}
            data={[
              { value: "monthly", label: "Monthly" },
              { value: "weekly", label: "Weekly" },
            ]}
            w={120}
            aria-label="Time-series interval"
          />
        </Group>
      </Group>

      <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
        <Card withBorder padding="md" radius="md">
          <Title order={5}>Severity Distribution</Title>
          {severityDist.length > 0 ? (
            <DonutChart
              mt="sm"
              data={severityDist.map((d, i) => ({
                name: d.label,
                value: Number(d.value),
                color: severityColors[i % severityColors.length],
              }))}
              size={160}
              thickness={20}
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Top Categories</Title>
          {topCategories.length > 0 ? (
            <BarChart
              mt="sm"
              h={160}
              data={topCategories.map((d) => ({
                category: d.label,
                count: Number(d.value),
              }))}
              dataKey="category"
              series={[{ name: "count", color: "blue" }]}
              orientation="vertical"
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Remediation Rate</Title>
          {remediation ? (
            <Stack gap="xs" mt="sm">
              <Group justify="space-between">
                <Text size="sm">Total Reported</Text>
                <Text size="sm" fw={600}>
                  {remediation.total}
                </Text>
              </Group>
              <Group justify="space-between">
                <Text size="sm">Fixed On Time</Text>
                <Text size="sm" fw={600} c="green">
                  {remediation.fixed_on_time}
                </Text>
              </Group>
              <Group justify="space-between">
                <Text size="sm">Overdue</Text>
                <Text size="sm" fw={600} c="red">
                  {remediation.overdue}
                </Text>
              </Group>
            </Stack>
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Findings Over Time</Title>
          {findingsTime.length > 0 ? (
            <LineChart
              mt="sm"
              h={160}
              data={findingsTime.map((d) => ({
                period: d.period,
                critical: d.critical,
                high: d.high,
                medium: d.medium,
                low: d.low,
                informational: d.informational,
              }))}
              dataKey="period"
              series={[
                { name: "critical", color: "red" },
                { name: "high", color: "orange" },
                { name: "medium", color: "yellow" },
                { name: "low", color: "blue" },
                { name: "informational", color: "gray" },
              ]}
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Engagement Timeline</Title>
          {timeline.length > 0 ? (
            <Stack gap="xs" mt="sm">
              {timeline.map((t) => (
                <Group key={t.engagement_id} justify="space-between">
                  <Text size="sm">{t.name}</Text>
                  <Text size="xs" c="dimmed">
                    {t.start_date || "—"} → {t.end_date || "—"}
                  </Text>
                </Group>
              ))}
            </Stack>
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Time by Activity</Title>
          {timeActivity.length > 0 ? (
            <DonutChart
              mt="sm"
              data={timeActivity.map((d, i) => ({
                name: d.label,
                value: Number(d.value),
                color: severityColors[i % severityColors.length],
              }))}
              size={160}
              thickness={20}
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Revenue by Client</Title>
          {revenue.length > 0 ? (
            <BarChart
              mt="sm"
              h={160}
              data={revenue.map((d) => ({
                client: d.label,
                revenue: Number(d.value),
              }))}
              dataKey="client"
              series={[{ name: "revenue", color: "teal" }]}
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No paid invoices yet
            </Text>
          )}
        </Card>

        <Card withBorder padding="md" radius="md">
          <Title order={5}>Budget vs Actual</Title>
          {budget.length > 0 ? (
            <BarChart
              mt="sm"
              h={160}
              data={budget.map((d) => ({
                engagement: d.name,
                budgeted: d.budgeted,
                actual: d.actual,
              }))}
              dataKey="engagement"
              series={[
                { name: "budgeted", color: "blue" },
                { name: "actual", color: "orange" },
              ]}
            />
          ) : (
            <Text c="dimmed" size="sm" mt="sm">
              No data
            </Text>
          )}
        </Card>
      </SimpleGrid>
    </Stack>
  );
}
