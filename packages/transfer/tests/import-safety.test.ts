import { expect, test } from "bun:test";

test("imports without browser or host globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const transfer = await import("@inflatable-cookie/longhorn-transfer");
  expect(transfer.TRANSFER_PROTOCOL_VERSION).toBe(1);
});
