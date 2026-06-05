import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Checkbox,
  Group,
  Select,
  Stack,
  Text,
  Title,
} from "@mantine/core";
import {
  listFrameworks,
  listControls,
  mapFindingToControl,
  getFindingMappings,
  removeComplianceMapping,
  type ComplianceFramework,
  type ComplianceControl,
  type FindingComplianceMapping,
} from "../api";

export function ComplianceView({ findingId }: { findingId: number }) {
  const [frameworks, setFrameworks] = useState<ComplianceFramework[]>([]);
  const [controls, setControls] = useState<ComplianceControl[]>([]);
  const [mappings, setMappings] = useState<FindingComplianceMapping[]>([]);
  const [selectedFramework, setSelectedFramework] = useState<number | null>(
    null,
  );

  const load = async () => {
    try {
      const [fw, mp] = await Promise.all([
        listFrameworks(),
        getFindingMappings(findingId),
      ]);
      setFrameworks(fw);
      setMappings(mp);
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    load();
  }, [findingId]);

  useEffect(() => {
    if (!selectedFramework) {
      setControls([]);
      return;
    }
    listControls(selectedFramework).then(setControls).catch(console.error);
  }, [selectedFramework]);

  const handleMap = async (controlId: number) => {
    try {
      await mapFindingToControl({
        finding_id: findingId,
        control_id: controlId,
      });
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const handleRemove = async (mappingId: number) => {
    try {
      await removeComplianceMapping(mappingId);
      await load();
    } catch (e) {
      console.error(e);
    }
  };

  const mappedControlIds = new Set(mappings.map((m) => m.control_id));

  return (
    <Stack gap="md">
      <Title order={5}>Compliance Mapping</Title>

      {mappings.length > 0 && (
        <Stack gap="xs">
          {mappings.map((m) => (
            <Card key={m.id} withBorder padding="xs" radius="md">
              <Group justify="space-between">
                <Stack gap={0}>
                  <Text size="sm" fw={600}>
                    {m.control.framework_name} · {m.control.control_id}
                  </Text>
                  <Text size="xs">{m.control.title}</Text>
                </Stack>
                <Button
                  size="xs"
                  variant="subtle"
                  color="red"
                  onClick={() => handleRemove(m.id)}
                >
                  Remove
                </Button>
              </Group>
            </Card>
          ))}
        </Stack>
      )}

      <Select
        placeholder="Select framework"
        data={frameworks.map((f) => ({ value: String(f.id), label: f.name }))}
        value={selectedFramework ? String(selectedFramework) : null}
        onChange={(v) => setSelectedFramework(v ? Number(v) : null)}
      />

      {controls.length > 0 && (
        <Stack gap="xs">
          {controls.map((c) => (
            <Group key={c.id} justify="space-between">
              <Checkbox
                label={`${c.control_id} · ${c.title}`}
                checked={mappedControlIds.has(c.id)}
                onChange={(e) => {
                  if (e.currentTarget.checked) {
                    handleMap(c.id);
                  } else {
                    const mapping = mappings.find((m) => m.control_id === c.id);
                    if (mapping) handleRemove(mapping.id);
                  }
                }}
              />
            </Group>
          ))}
        </Stack>
      )}
    </Stack>
  );
}
