import {
  Badge,
  Drawer,
  Group,
  Stack,
  Text,
  Title,
  Paper,
  Divider,
} from "@mantine/core";
import type { Finding, Severity } from "../api/findings";

const SEVERITY_COLORS: Record<Severity, string> = {
  critical: "red",
  high: "orange",
  medium: "yellow",
  low: "blue",
  informational: "gray",
};

interface Props {
  finding: Finding | null;
  opened: boolean;
  onClose: () => void;
}

export function FindingDetail({ finding, opened, onClose }: Props) {
  if (!finding) return null;

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={finding.title}
      position="right"
      size="xl"
    >
      <Stack gap="md">
        <Group>
          <Badge color={SEVERITY_COLORS[finding.severity]}>
            {finding.severity}
          </Badge>
          <Badge variant="outline">{finding.status}</Badge>
          {finding.cvss_score !== null && (
            <Badge color="grape">CVSS {finding.cvss_score}</Badge>
          )}
          {finding.owasp_category && (
            <Badge color="teal">{finding.owasp_category}</Badge>
          )}
          {finding.cwe_id && <Badge color="pink">{finding.cwe_id}</Badge>}
        </Group>

        <Text size="sm" c="dimmed">
          {finding.client_name} · {finding.engagement_name}
        </Text>

        <Divider />

        <Paper withBorder p="sm">
          <Title order={5}>Overview</Title>
          <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
            {finding.overview}
          </Text>
        </Paper>

        <Paper withBorder p="sm">
          <Title order={5}>Summary</Title>
          <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
            {finding.summary}
          </Text>
        </Paper>

        <Paper withBorder p="sm">
          <Title order={5}>Steps to Reproduce</Title>
          <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
            {finding.steps_to_reproduce}
          </Text>
        </Paper>

        {finding.affected_endpoints.length > 0 && (
          <Paper withBorder p="sm">
            <Title order={5}>Affected Endpoints</Title>
            <Stack gap="xs" mt="xs">
              {finding.affected_endpoints.map((ep, i) => (
                <Text key={i} size="sm">
                  <strong>
                    {ep.method} {ep.path}
                  </strong>
                  <br />
                  {ep.description}
                </Text>
              ))}
            </Stack>
          </Paper>
        )}

        {finding.evidence.length > 0 && (
          <Paper withBorder p="sm">
            <Title order={5}>Evidence</Title>
            <Stack gap="xs" mt="xs">
              {finding.evidence.map((ev, i) => (
                <Stack key={i} gap={2}>
                  <Text fw={600} size="sm">
                    {ev.title || `Evidence #${i + 1}`}
                  </Text>
                  <Text size="xs" c="dimmed" style={{ whiteSpace: "pre-wrap" }}>
                    Request:
                    <br />
                    {ev.request}
                  </Text>
                  <Text size="xs" c="dimmed" style={{ whiteSpace: "pre-wrap" }}>
                    Response:
                    <br />
                    {ev.response}
                  </Text>
                </Stack>
              ))}
            </Stack>
          </Paper>
        )}

        {finding.impact_items.length > 0 && (
          <Paper withBorder p="sm">
            <Title order={5}>Impact</Title>
            <Stack gap="xs" mt="xs">
              {finding.impact_items.map((item, i) => (
                <Text key={i} size="sm">
                  <strong>{item.title}</strong>
                  <br />
                  {item.explanation}
                </Text>
              ))}
            </Stack>
          </Paper>
        )}

        {finding.remediation_items.length > 0 && (
          <Paper withBorder p="sm">
            <Title order={5}>Remediation</Title>
            <Stack gap="xs" mt="xs">
              {finding.remediation_items.map((item, i) => (
                <Text key={i} size="sm">
                  <strong>{item.action}</strong>
                  <br />
                  {item.fix}
                  {item.code_snippet && (
                    <pre style={{ fontSize: 12 }}>{item.code_snippet}</pre>
                  )}
                </Text>
              ))}
            </Stack>
          </Paper>
        )}

        {finding.references.length > 0 && (
          <Paper withBorder p="sm">
            <Title order={5}>References</Title>
            <Stack gap="xs" mt="xs">
              {finding.references.map((ref, i) => (
                <Text key={i} size="sm">
                  <a href={ref.url} target="_blank" rel="noreferrer">
                    {ref.title}
                  </a>
                </Text>
              ))}
            </Stack>
          </Paper>
        )}

        {finding.tags.length > 0 && (
          <Group gap="xs">
            {finding.tags.map((tag) => (
              <Badge key={tag} size="sm" variant="outline">
                {tag}
              </Badge>
            ))}
          </Group>
        )}

        {finding.notes && (
          <Paper withBorder p="sm">
            <Title order={5}>Notes</Title>
            <Text size="sm" style={{ whiteSpace: "pre-wrap" }}>
              {finding.notes}
            </Text>
          </Paper>
        )}
      </Stack>
    </Drawer>
  );
}
