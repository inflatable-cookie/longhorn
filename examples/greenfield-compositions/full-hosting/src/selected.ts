export type SelectedContracts = {
  commands: typeof import("@longhorn/commands");
  config: typeof import("@longhorn/config");
  core: typeof import("@longhorn/core");
  history: typeof import("@longhorn/history");
  layout: typeof import("@longhorn/layout");
  poodle: typeof import("@longhorn/poodle");
  settings: typeof import("@longhorn/settings");
  surfaceTransfer: typeof import("@longhorn/surface-transfer");
  surfaces: typeof import("@longhorn/surfaces");
  svelte: typeof import("@longhorn/svelte");
  tauri: typeof import("@longhorn/tauri");
  transfer: typeof import("@longhorn/transfer");
};

export const selectedModules = [
  "@longhorn/commands",
  "@longhorn/config",
  "@longhorn/core",
  "@longhorn/history",
  "@longhorn/layout",
  "@longhorn/poodle",
  "@longhorn/settings",
  "@longhorn/surface-transfer",
  "@longhorn/surfaces",
  "@longhorn/svelte",
  "@longhorn/tauri",
  "@longhorn/transfer",
] as const;
