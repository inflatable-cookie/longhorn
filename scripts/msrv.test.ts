import { describe, expect, test } from "bun:test";
import { MSRV, MSRV_TOOLCHAIN, parseWorkspaceRustVersion } from "./msrv.ts";

describe("msrv", () => {
  test("exposes the declared MSRV floor", () => {
    expect(MSRV).toMatch(/^\d+\.\d+\.\d+$/);
    expect(MSRV_TOOLCHAIN).toBe(MSRV);
  });

  test("parses [workspace.package].rust-version instead of matching comments or other tables", () => {
    const manifest = [
      '# rust-version = "1.97.1"',
      "[workspace]",
      "members = []",
      "",
      "[package]",
      'rust-version = "1.97.1"',
      "",
      "[workspace.package]",
      'rust-version = "1.85.0"',
    ].join("\n");
    expect(parseWorkspaceRustVersion(manifest)).toBe("1.85.0");

    const missing = ["[workspace]", "members = []", '# rust-version = "1.97.1"'].join("\n");
    expect(parseWorkspaceRustVersion(missing)).toBeUndefined();
  });
});
