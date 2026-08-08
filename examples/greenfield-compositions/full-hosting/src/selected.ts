export type SelectedContracts = {
  commands: typeof import("@inflatable-cookie/longhorn-commands");
  config: typeof import("@inflatable-cookie/longhorn-config");
  core: typeof import("@inflatable-cookie/longhorn-core");
  history: typeof import("@inflatable-cookie/longhorn-history");
  layout: typeof import("@inflatable-cookie/longhorn-layout");
  poodle: typeof import("@inflatable-cookie/longhorn-poodle");
  settings: typeof import("@inflatable-cookie/longhorn-settings");
  surfaceTransfer: typeof import("@inflatable-cookie/longhorn-surface-transfer");
  surfaces: typeof import("@inflatable-cookie/longhorn-surfaces");
  svelte: typeof import("@inflatable-cookie/longhorn-svelte");
  tauri: typeof import("@inflatable-cookie/longhorn-tauri");
  transfer: typeof import("@inflatable-cookie/longhorn-transfer");
};

export const selectedModules = [
  "@inflatable-cookie/longhorn-commands",
  "@inflatable-cookie/longhorn-config",
  "@inflatable-cookie/longhorn-core",
  "@inflatable-cookie/longhorn-history",
  "@inflatable-cookie/longhorn-layout",
  "@inflatable-cookie/longhorn-poodle",
  "@inflatable-cookie/longhorn-settings",
  "@inflatable-cookie/longhorn-surface-transfer",
  "@inflatable-cookie/longhorn-surfaces",
  "@inflatable-cookie/longhorn-svelte",
  "@inflatable-cookie/longhorn-tauri",
  "@inflatable-cookie/longhorn-transfer",
] as const;
