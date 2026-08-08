import { expect, test } from "bun:test";

test("root imports without browser or optional-system globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const settings = await import("@inflatable-cookie/longhorn/settings");
  expect(settings.SETTINGS_PROTOCOL_VERSION).toBe(1);
  expect(settings.SettingsClient).toBeFunction();
  expect(settings.projectSettingsRegistry).toBeFunction();
});
