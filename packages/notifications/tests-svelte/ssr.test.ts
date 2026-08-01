import { describe, expect, it } from "vitest";

describe("notification optional modules", () => {
  it("imports without browser globals during SSR", async () => {
    expect("window" in globalThis).toBe(false);
    expect("document" in globalThis).toBe(false);
    const root = await import("../src/index.ts");
    const svelte = await import("../src/svelte.ts");
    const poodle = await import("../src/poodle.ts");
    expect(root.NotificationController).toBeTypeOf("function");
    expect(svelte.NotificationSession).toBeTypeOf("function");
    expect(poodle.NotificationPanel).toBeTruthy();
  }, 20_000);
});
