import type { ComponentType } from "react";

export interface ExtensionManifest {
  id: string;
  name: string;
  description: string;
  icon: string;
  component: ComponentType<any>;
  navLabel: string;
  category: "deliverables" | "tools";
}

export interface ExtensionState {
  enabled: boolean;
  manifest: ExtensionManifest;
}
