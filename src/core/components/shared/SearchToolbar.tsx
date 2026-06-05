import { Group, TextInput, Select } from "@mantine/core";
import { IconSearch } from "@tabler/icons-react";

interface SearchToolbarProps {
  search: string;
  onSearchChange: (value: string) => void;
  filters?: {
    value: string | null;
    onChange: (value: string | null) => void;
    options: { value: string; label: string }[];
    placeholder?: string;
  }[];
}

export function SearchToolbar({
  search,
  onSearchChange,
  filters = [],
}: SearchToolbarProps) {
  return (
    <Group gap="sm" align="flex-end">
      <TextInput
        placeholder="Search..."
        leftSection={<IconSearch size={16} />}
        value={search}
        onChange={(e) => onSearchChange(e.currentTarget.value)}
        style={{ flex: 1 }}
      />
      {filters.map((f, i) => (
        <Select
          key={i}
          placeholder={f.placeholder || "Filter"}
          data={f.options}
          value={f.value}
          onChange={f.onChange}
          clearable
          style={{ width: 160 }}
        />
      ))}
    </Group>
  );
}
