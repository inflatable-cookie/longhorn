import { expect, test } from "bun:test";

test("imports without browser or host globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const surfaceTransfer = await import("@longhorn/surface-transfer");
  expect(surfaceTransfer.SURFACE_TRANSFER_TARGET_KINDS).toEqual([
    "existing",
    "provisioned",
  ]);
});
