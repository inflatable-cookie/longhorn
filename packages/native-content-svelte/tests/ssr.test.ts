import { describe, expect, it } from "vitest";

describe("@longhorn/native-content-svelte SSR boundary", () => {
  it("imports without browser globals", async () => {
    const adapter = await import("../src/index.ts");
    expect(adapter.NativeContentSession).toBeTruthy();
    expect(adapter.nativeContentViewport).toBeTypeOf("function");
  });
});
