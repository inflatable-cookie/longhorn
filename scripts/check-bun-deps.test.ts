import { describe, expect, test } from "bun:test";

import { checkBunDependencies } from "./check-bun-deps.ts";

describe("check:bun-deps", () => {
  test("runs the locked install once, then passes when markers appear", async () => {
    let installed = false;
    let installCalls = 0;
    const messages: string[] = [];

    const passed = await checkBunDependencies({
      root: "/fixture",
      markerExists: async () => installed,
      install: async () => {
        installCalls += 1;
        installed = true;
        return 0;
      },
      log: (message) => messages.push(message),
    });

    expect(passed).toBe(true);
    expect(installCalls).toBe(1);
    expect(messages[0]).toBe("Bun dependencies missing; running bun install --frozen-lockfile.");
    expect(messages[1]).toContain('"outcome": "pass"');
  });

  test("fails when the frozen install fails", async () => {
    const errors: string[] = [];
    let installCalls = 0;

    const passed = await checkBunDependencies({
      root: "/fixture",
      markerExists: async () => false,
      install: async () => {
        installCalls += 1;
        return 2;
      },
      log: () => {},
      error: (message) => errors.push(message),
    });

    expect(passed).toBe(false);
    expect(installCalls).toBe(1);
    expect(errors).toEqual(["Frozen Bun install failed with exit code 2."]);
  });

  test("fails when markers are still missing after a successful install", async () => {
    const errors: string[] = [];
    let markerChecks = 0;

    const passed = await checkBunDependencies({
      root: "/fixture",
      markerExists: async () => {
        markerChecks += 1;
        return false;
      },
      install: async () => 0,
      log: () => {},
      error: (message) => errors.push(message),
    });

    expect(passed).toBe(false);
    expect(markerChecks).toBe(6);
    expect(errors[0]).toContain("dependency markers are still missing");
  });
});
