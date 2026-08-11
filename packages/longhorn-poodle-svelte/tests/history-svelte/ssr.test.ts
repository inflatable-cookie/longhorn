import { describe, expect, it } from "vitest";

describe("history optional modules", () => {
  it("import without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);

    const svelte = await import("../../src/history/svelte.ts");
    expect(svelte.HistorySession).toBeTypeOf("function");
  }, 60_000);
});
