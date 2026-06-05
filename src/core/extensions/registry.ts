import type { ExtensionManifest } from "./types";

const registry = new Map<string, ExtensionManifest>();

export function registerExtension(manifest: ExtensionManifest): void {
  registry.set(manifest.id, manifest);
}

export function getExtension(id: string): ExtensionManifest | undefined {
  return registry.get(id);
}

export function getAllExtensions(): ExtensionManifest[] {
  return Array.from(registry.values());
}

export function getExtensionsByCategory(category: ExtensionManifest["category"]): ExtensionManifest[] {
  return getAllExtensions().filter((ext) => ext.category === category);
}
