export type SelectedContracts = {
  commands: typeof import("@longhorn/commands");
  config: typeof import("@longhorn/config");
  core: typeof import("@longhorn/core");
  layout: typeof import("@longhorn/layout");
  poodle: typeof import("@longhorn/poodle");
  settings: typeof import("@longhorn/settings");
  svelte: typeof import("@longhorn/svelte");
  tauri: typeof import("@longhorn/tauri");
};

export const selectedModules = [
  "@longhorn/commands",
  "@longhorn/config",
  "@longhorn/core",
  "@longhorn/layout",
  "@longhorn/poodle",
  "@longhorn/settings",
  "@longhorn/svelte",
  "@longhorn/tauri",
] as const;
