import { expect, test } from "bun:test";

test("root imports without browser, Tauri, Svelte, Poodle, or execution globals", async () => {
  expect("window" in globalThis).toBeFalse();
  expect("document" in globalThis).toBeFalse();
  expect("__TAURI_INTERNALS__" in globalThis).toBeFalse();

  const commands = await import("@inflatable-cookie/longhorn/commands");
  expect(commands.COMMAND_KEYMAP_PROTOCOL_VERSION).toBe(1);
  expect("executeCommand" in commands).toBeFalse();
});
