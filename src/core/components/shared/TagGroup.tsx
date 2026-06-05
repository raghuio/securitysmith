import { Badge, Group } from "@mantine/core";

interface TagGroupProps {
  tags: string[];
  color?: string;
  size?: string;
}

export function TagGroup({ tags, color = "blue", size = "sm" }: TagGroupProps) {
  if (!tags || tags.length === 0) return null;
  return (
    <Group gap={4}>
      {tags.map((tag) => (
        <Badge key={tag} color={color} variant="light" size={size}>
          #{tag}
        </Badge>
      ))}
    </Group>
  );
}
