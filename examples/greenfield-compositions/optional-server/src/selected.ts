export type SelectedContracts = {
  bridge: typeof import("@longhorn/bridge");
  config: typeof import("@longhorn/config");
  core: typeof import("@longhorn/core");
  settings: typeof import("@longhorn/settings");
  tauri: typeof import("@longhorn/tauri");
};

export const selectedModules = [
  "@longhorn/bridge",
  "@longhorn/config",
  "@longhorn/core",
  "@longhorn/settings",
  "@longhorn/tauri",
] as const;
