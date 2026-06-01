export interface DashboardWidgets {
  clients: boolean;
  engagements: boolean;
  findings: boolean;
  credentials: boolean;
  news: boolean;
  calendar: boolean;
  recent_activity: boolean;
}

export interface NavItems {
  dashboard: boolean;
  clients: boolean;
  engagements: boolean;
  templates: boolean;
  documents: boolean;
  calendar: boolean;
  news: boolean;
  activity_log: boolean;
  settings: boolean;
}

export const PREDEFINED_COLORS = [
  { name: "Blue", hex: "#228be6" },
  { name: "Teal", hex: "#12b886" },
  { name: "Green", hex: "#40c057" },
  { name: "Orange", hex: "#fd7e14" },
  { name: "Red", hex: "#fa5252" },
  { name: "Purple", hex: "#7950f2" },
  { name: "Pink", hex: "#e64980" },
  { name: "Gray", hex: "#868e96" },
] as const;
