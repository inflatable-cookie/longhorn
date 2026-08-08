import { describe, expect, it } from "vitest";

describe("settings optional modules", () => {
  it("import without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);

    const svelte = await import("../../src/settings/svelte.ts");
    const poodle = await import("../../src/settings/poodle.ts");
    expect(svelte.SettingsSession).toBeTypeOf("function");
    expect(poodle.SettingsShell).toBeTruthy();
  }, 20_000);
});
