import { expect, test } from "bun:test";

test("root imports without browser, Tauri, Svelte, or Poodle globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const config = await import("@inflatable-cookie/longhorn-config");
  expect(config.CONFIG_OPERATIONS_PROTOCOL_VERSION).toBe(1);
  expect(config.ConfigOperationsClient).toBeFunction();
});
