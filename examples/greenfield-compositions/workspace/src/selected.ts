export type SelectedContracts = {
  commands: typeof import("@inflatable-cookie/longhorn-commands");
  config: typeof import("@inflatable-cookie/longhorn-config");
  core: typeof import("@inflatable-cookie/longhorn-core");
  layout: typeof import("@inflatable-cookie/longhorn-layout");
  poodle: typeof import("@inflatable-cookie/longhorn-poodle");
  settings: typeof import("@inflatable-cookie/longhorn-settings");
  svelte: typeof import("@inflatable-cookie/longhorn-svelte");
  tauri: typeof import("@inflatable-cookie/longhorn-tauri");
};

export const selectedModules = [
  "@inflatable-cookie/longhorn-commands",
  "@inflatable-cookie/longhorn-config",
  "@inflatable-cookie/longhorn-core",
  "@inflatable-cookie/longhorn-layout",
  "@inflatable-cookie/longhorn-poodle",
  "@inflatable-cookie/longhorn-settings",
  "@inflatable-cookie/longhorn-svelte",
  "@inflatable-cookie/longhorn-tauri",
] as const;
