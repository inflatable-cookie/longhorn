import { expect, test } from "bun:test";

test("imports without reading browser globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();

  const adapter = await import("@longhorn/tauri");
  expect(adapter.TauriTransport).toBeFunction();
});
