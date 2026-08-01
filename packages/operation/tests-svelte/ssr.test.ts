import { describe, expect, it } from "vitest";

describe("operation optional modules", () => {
  it("import without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);

    const root = await import("../src/index.ts");
    const svelte = await import("../src/svelte.ts");
    const poodle = await import("../src/poodle.ts");
    expect(root.OperationController).toBeTypeOf("function");
    expect(svelte.OperationSession).toBeTypeOf("function");
    expect(poodle.OperationPanel).toBeTruthy();
  }, 20_000);
});
