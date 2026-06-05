import { registerExtension } from "../core/extensions/registry";
import type { ExtensionManifest } from "../core/extensions/types";

// Deliverables
registerExtension({
  id: "templates",
  name: "Templates",
  description: "Report and document templates",
  icon: "IconTemplate",
  component: () => null, // Will be lazy-loaded
  navLabel: "Templates",
  category: "deliverables",
} as ExtensionManifest);

registerExtension({
  id: "reports",
  name: "Reports",
  description: "Generate security reports from findings",
  icon: "IconReport",
  component: () => null,
  navLabel: "Reports",
  category: "deliverables",
} as ExtensionManifest);

registerExtension({
  id: "documents",
  name: "Documents",
  description: "Document generation (SOWs, proposals)",
  icon: "IconFileText",
  component: () => null,
  navLabel: "Documents",
  category: "deliverables",
} as ExtensionManifest);

registerExtension({
  id: "invoices",
  name: "Invoices",
  description: "Invoice creation and management",
  icon: "IconReceipt",
  component: () => null,
  navLabel: "Invoices",
  category: "deliverables",
} as ExtensionManifest);

// Tools
registerExtension({
  id: "news",
  name: "News",
  description: "Security news aggregator",
  icon: "IconNews",
  component: () => null,
  navLabel: "News",
  category: "tools",
} as ExtensionManifest);

registerExtension({
  id: "calendar",
  name: "Calendar",
  description: "Due dates, reminders, scheduling",
  icon: "IconCalendar",
  component: () => null,
  navLabel: "Calendar",
  category: "tools",
} as ExtensionManifest);

registerExtension({
  id: "analytics",
  name: "Analytics",
  description: "Charts, metrics, performance stats",
  icon: "IconChartBar",
  component: () => null,
  navLabel: "Analytics",
  category: "tools",
} as ExtensionManifest);

registerExtension({
  id: "checklists",
  name: "Checklists",
  description: "Methodology checklists",
  icon: "IconListCheck",
  component: () => null,
  navLabel: "Checklists",
  category: "tools",
} as ExtensionManifest);

registerExtension({
  id: "compliance",
  name: "Compliance",
  description: "Compliance mapping and tracking",
  icon: "IconShieldCheck",
  component: () => null,
  navLabel: "Compliance",
  category: "tools",
} as ExtensionManifest);


registerExtension({
  id: "email",
  name: "Email",
  description: "SMTP config and email composer",
  icon: "IconMail",
  component: () => null,
  navLabel: "Email",
  category: "tools",
} as ExtensionManifest);

registerExtension({
  id: "time-tracking",
  name: "Time Tracking",
  description: "Time entries for engagements",
  icon: "IconClock",
  component: () => null,
  navLabel: "Time Tracking",
  category: "tools",
} as ExtensionManifest);
