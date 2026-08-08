import { describe, expect, it } from "vitest";

describe("command optional modules", () => {
  it("import without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);

    const svelte = await import("../../src/commands/svelte.ts");
    const poodle = await import("../../src/commands/poodle.ts");
    expect(svelte.CommandSession).toBeTypeOf("function");
    expect(poodle.CommandPaletteBinding).toBeTruthy();
    expect(poodle.KeybindingSettings).toBeTruthy();
  }, 20_000);
});
