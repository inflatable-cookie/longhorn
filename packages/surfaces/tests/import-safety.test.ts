import { expect, test } from "bun:test";

test("imports without browser or host globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const surfaces = await import("@inflatable-cookie/longhorn-surfaces");
  expect(surfaces.SURFACE_PROTOCOL_VERSION).toBe(1);
});
