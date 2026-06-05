// Extension imports — static for desktop app bundling
import { TemplateLibrary } from "./templates/components/TemplateLibrary";
import { ReportList } from "./reports/components/ReportList";
import { DocumentList } from "./documents/components/DocumentList";
import { InvoiceList } from "./invoices/components/InvoiceList";
import { NewsFeed } from "./news/components/NewsFeed";
import { CalendarView } from "./calendar/components/CalendarView";
import { AnalyticsPanel } from "./analytics/components/AnalyticsPanel";
import { ChecklistEditor } from "./checklists/components/ChecklistView";
import { ComplianceView } from "./compliance/components/ComplianceView";
import { AiChat } from "./ai/components/AiChat";
import { EmailComposer } from "./email/components/EmailComposer";
import { TimeTracker } from "./time-tracking/components/TimeTracker";

import type { ExtensionManifest } from "../core/extensions/types";

// Toggleable extensions — user can enable/disable in Settings
export const extensions: ExtensionManifest[] = [
  { id: "templates", name: "Templates", description: "Report templates", icon: "IconTemplate", navLabel: "Templates", category: "deliverables", component: TemplateLibrary },
  { id: "reports", name: "Reports", description: "Report generation", icon: "IconReport", navLabel: "Reports", category: "deliverables", component: ReportList },
  { id: "documents", name: "Documents", description: "Document builder", icon: "IconFileText", navLabel: "Documents", category: "deliverables", component: DocumentList },
  { id: "invoices", name: "Invoices", description: "Invoice management", icon: "IconReceipt", navLabel: "Invoices", category: "deliverables", component: InvoiceList },
  { id: "news", name: "News", description: "Security news", icon: "IconNews", navLabel: "News", category: "tools", component: NewsFeed },
  { id: "calendar", name: "Calendar", description: "Calendar and reminders", icon: "IconCalendar", navLabel: "Calendar", category: "tools", component: CalendarView },
  { id: "analytics", name: "Analytics", description: "Charts and stats", icon: "IconChartBar", navLabel: "Analytics", category: "tools", component: AnalyticsPanel },
  { id: "checklists", name: "Checklists", description: "Methodology checklists", icon: "IconListCheck", navLabel: "Checklists", category: "tools", component: ChecklistEditor },
  { id: "compliance", name: "Compliance", description: "Compliance mapping", icon: "IconShieldCheck", navLabel: "Compliance", category: "tools", component: ComplianceView },
  { id: "email", name: "Email", description: "Email composer", icon: "IconMail", navLabel: "Email", category: "tools", component: EmailComposer },
  { id: "time-tracking", name: "Time Tracking", description: "Time entries", icon: "IconClock", navLabel: "Time Tracking", category: "tools", component: TimeTracker },
];

export { TemplateLibrary, ReportList, DocumentList, InvoiceList, NewsFeed, CalendarView, AnalyticsPanel, ChecklistEditor, ComplianceView, AiChat, EmailComposer, TimeTracker };
