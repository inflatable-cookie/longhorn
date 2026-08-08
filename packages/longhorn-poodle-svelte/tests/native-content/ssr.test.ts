import { describe, expect, it } from "vitest";

describe("@inflatable-cookie/longhorn-poodle-svelte/native-content SSR boundary", () => {
  it("imports without browser globals", async () => {
    const adapter = await import("../../src/native-content/index.ts");
    expect(adapter.NativeContentSession).toBeTruthy();
    expect(adapter.nativeContentViewport).toBeTypeOf("function");
  });
});
