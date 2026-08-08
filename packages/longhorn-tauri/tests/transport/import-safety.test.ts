import { expect, test } from "bun:test";

test("imports without reading browser globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();

  const adapter = await import("@inflatable-cookie/longhorn-tauri");
  expect(adapter.TauriTransport).toBeFunction();
});
