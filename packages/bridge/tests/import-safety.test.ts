import { expect, test } from "bun:test";

test("root imports without browser, host, service, or event globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const bridge = await import("@longhorn/bridge");
  expect(bridge.BRIDGE_PROTOCOL_VERSION).toBe(1);
  expect("connectBridgeStream" in bridge).toBeFalse();
  expect("DirectBridgeStreamSource" in bridge).toBeFalse();
});
