import { Badge } from "@mantine/core";
import {
  severity,
  engagementStatus,
  invoiceStatus,
  documentStatus,
  findingStatus,
} from "../../theme/tokens";

type StatusMap = Record<string, { color: string; label: string }>;

const STATUS_MAPS: Record<string, StatusMap> = {
  severity,
  engagement: engagementStatus,
  invoice: invoiceStatus,
  document: documentStatus,
  finding: findingStatus,
};

interface StatusBadgeProps {
  type: "severity" | "engagement" | "invoice" | "document" | "finding";
  value: string;
  size?: string;
}

export function StatusBadge({ type, value, size }: StatusBadgeProps) {
  const map = STATUS_MAPS[type];
  const config = map?.[value];
  if (!config) {
    return (
      <Badge variant="default" size={size}>
        {value}
      </Badge>
    );
  }
  return (
    <Badge color={config.color} variant="light" size={size}>
      {config.label}
    </Badge>
  );
}
