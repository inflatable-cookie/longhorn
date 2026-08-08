export type SelectedContracts = {
  config: typeof import("@inflatable-cookie/longhorn-config");
  core: typeof import("@inflatable-cookie/longhorn-core");
  settings: typeof import("@inflatable-cookie/longhorn-settings");
  tauri: typeof import("@inflatable-cookie/longhorn-tauri");
};

export const selectedModules = [
  "@inflatable-cookie/longhorn-config",
  "@inflatable-cookie/longhorn-core",
  "@inflatable-cookie/longhorn-settings",
  "@inflatable-cookie/longhorn-tauri",
] as const;
