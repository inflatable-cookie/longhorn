import { expect, test } from "bun:test";

test("root imports without browser, Tauri, Svelte, Poodle, or product globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const history = await import("@inflatable-cookie/longhorn/history");
  expect(history.HISTORY_PROTOCOL_VERSION).toBe(1);
  expect("HistorySession" in history).toBeFalse();
  expect("HistoryPanel" in history).toBeFalse();
  expect("createTauriHistoryPort" in history).toBeFalse();
});
