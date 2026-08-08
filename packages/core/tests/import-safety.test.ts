import { expect, test } from "bun:test";

test("imports without browser or host globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const core = await import("@inflatable-cookie/longhorn-core");
  expect(core.CheckedSnapshotConnection).toBeFunction();
  expect(core.isEventTransport).toBeFunction();
});
