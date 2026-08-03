import { describe, expect, it } from "vitest";
describe("fork-history package boundaries", () => { it("root imports without browser or optional UI globals", async () => { expect("window" in globalThis).toBe(false); const root = await import("../src/index.ts"); expect(root.ForkHistoryClient).toBeTypeOf("function"); }); });
