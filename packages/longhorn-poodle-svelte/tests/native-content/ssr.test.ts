import { describe, expect, it } from "vitest";

describe("@inflatable-cookie/longhorn-poodle-svelte/native-content SSR boundary", () => {
  it("imports without browser globals", async () => {
    // This suite runs in the node environment, so window/document must be
    // absent. Assert that up front so the import below proves the
    // native-content entry pulls in no browser globals instead of
    // incidentally passing if the environment ever changes.
    expect((globalThis as { window?: unknown }).window).toBeUndefined();
    expect((globalThis as { document?: unknown }).document).toBeUndefined();

    // Keep the dynamic import inside the timed section: the 60 s budget
    // covers cold Vite transform and collection while a full `effigy qa`
    // compiles the Rust workspace on the same machine, not the import
    // itself (the module graph is types plus `onMount` and imports fast
    // warm). Scoped here, not global, so slow client suites still fail fast.
    const adapter = await import("../../src/native-content/index.ts");
    expect(adapter.NativeContentSession).toBeTruthy();
    expect(adapter.nativeContentViewport).toBeTypeOf("function");
  }, 60_000);
});
