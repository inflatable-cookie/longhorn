import { describe, expect, it } from "vitest";

describe("history optional modules", () => {
  it("import without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);

    const svelte = await import("../../src/history/svelte.ts");
    const poodle = await import("../../src/history/poodle.ts");
    expect(svelte.HistorySession).toBeTypeOf("function");
    expect(poodle.HistoryPanel).toBeTruthy();
  }, 60_000);
});
