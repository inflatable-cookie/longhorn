export { default as StorageSettingsPage } from "./poodle/StorageSettingsPage.svelte";
export { default as BackupSettingsPage } from "./poodle/BackupSettingsPage.svelte";
export { default as RestoreSettingsPage } from "./poodle/RestoreSettingsPage.svelte";
export type { ConfigOperationsPageProps } from "./poodle/types.ts";

export const STORAGE_SETTINGS_RENDERER_ID = "longhorn:config.storage";
export const BACKUP_SETTINGS_RENDERER_ID = "longhorn:config.backup";
export const RESTORE_SETTINGS_RENDERER_ID = "longhorn:config.restore";
