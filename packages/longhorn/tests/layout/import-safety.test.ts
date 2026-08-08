import { expect, test } from "bun:test";

test("imports without browser or host globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const layout = await import("@inflatable-cookie/longhorn/layout");
  expect(layout.LAYOUT_PROTOCOL_VERSION).toBe(1);
});
