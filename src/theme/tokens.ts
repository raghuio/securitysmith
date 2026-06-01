export const severity = {
  critical: { color: "red.8", bg: "red.0", label: "Critical" },
  high: { color: "orange.7", bg: "orange.0", label: "High" },
  medium: { color: "yellow.7", bg: "yellow.0", label: "Medium" },
  low: { color: "blue.6", bg: "blue.0", label: "Low" },
  info: { color: "gray.6", bg: "gray.0", label: "Informational" },
} as const;

export const engagementStatus = {
  planned: { color: "gray", label: "Planned" },
  scheduled: { color: "violet", label: "Scheduled" },
  active: { color: "green", label: "Active" },
  paused: { color: "orange", label: "Paused" },
  completed: { color: "blue", label: "Completed" },
} as const;

export const invoiceStatus = {
  draft: { color: "gray", label: "Draft" },
  sent: { color: "blue", label: "Sent" },
  paid: { color: "green", label: "Paid" },
  cancelled: { color: "red", label: "Cancelled" },
  overdue: { color: "orange", label: "Overdue" },
} as const;

export const documentStatus = {
  draft: { color: "gray", label: "Draft" },
  finalized: { color: "green", label: "Finalized" },
} as const;

export const findingStatus = {
  draft: { color: "gray", label: "Draft" },
  confirmed: { color: "orange", label: "Confirmed" },
  reported: { color: "blue", label: "Reported" },
  fixed: { color: "green", label: "Fixed" },
  accepted: { color: "teal", label: "Accepted" },
  false_positive: { color: "red", label: "False Positive" },
  wont_fix: { color: "yellow", label: "Won't Fix" },
} as const;
