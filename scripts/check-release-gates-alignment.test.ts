import { afterAll, describe, expect, test } from "bun:test";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import {
  alignmentFailures,
  alignReleaseGates,
  checkReleaseGatesAlignment,
  parseReleaseGates,
  parseReleaseGatesSelector,
} from "./check-release-gates-alignment.ts";
import { releaseFixtureScratch } from "./fixture-scratch.ts";
import type { ReleaseFixtureScratch } from "./fixture-scratch.ts";

// One tracker per file: every fixture it creates is removed, and the suite
// fails if a `longhorn-release-*` directory is left behind (longhorn#174).
let scratch: ReleaseFixtureScratch | undefined;
afterAll(async () => {
  await scratch?.settle();
});

const releaseToml = `
[release]
changelog = "CHANGELOG.md"
# This is the only \`[release.gates]\` table; do not match this comment.

[release.gates]
private-candidate = "bun scripts/verify-private-candidate-docs.ts"
advisories = "cargo deny check advisories"
rustdoc = "effigy docs:rust"
prototypes = "effigy check:prototypes"
workspace = "effigy qa"
floor = "effigy release:floor"
source = "effigy release:source-consumer"
`;

const alignedEffigy = `
[tasks]
"release:gates" = [
  "bun scripts/verify-private-candidate-docs.ts",
  "cargo deny check advisories",
  { task = "docs:rust" },
  { task = "check:prototypes" },
  { task = "release:floor" },
  { task = "release:source-consumer" },
]
`;

describe("TOML reading", () => {
  test("a gate command with quotes and brackets is read as written", () => {
    const toml = [
      "[release.gates]",
      `docs = 'RUSTDOCFLAGS="-D warnings" effigy docs:rust'`,
      `index = "bash -c 'x[0]'"`,
      "",
    ].join("\n");
    expect(parseReleaseGates(toml)).toEqual([
      { name: "docs", command: 'RUSTDOCFLAGS="-D warnings" effigy docs:rust' },
      { name: "index", command: "bash -c 'x[0]'" },
    ]);
  });

  test("a release:gates entry containing a bracket is not cut short", () => {
    const toml = [
      "[tasks]",
      '"release:gates" = [',
      `  "bash -c 'x[0]'",`,
      '  { task = "docs:rust" },',
      "]",
      "",
    ].join("\n");
    expect(parseReleaseGatesSelector(toml)).toEqual(["bash -c 'x[0]'", "effigy docs:rust"]);
  });
});

describe("release:gates alignment", () => {
  test("equals [release.gates] minus workspace in declaration order", () => {
    const gates = parseReleaseGates(releaseToml);
    expect(gates.map((gate) => gate.name)).toEqual([
      "private-candidate",
      "advisories",
      "rustdoc",
      "prototypes",
      "workspace",
      "floor",
      "source",
    ]);
    expect(parseReleaseGatesSelector(alignedEffigy)).toEqual([
      "bun scripts/verify-private-candidate-docs.ts",
      "cargo deny check advisories",
      "effigy docs:rust",
      "effigy check:prototypes",
      "effigy release:floor",
      "effigy release:source-consumer",
    ]);
    expect(alignmentFailures(alignReleaseGates(releaseToml, alignedEffigy))).toEqual([]);
  });

  test("fails on a dropped or reordered gate", () => {
    const subset = `
[tasks]
"release:gates" = [
  { task = "release:floor" },
  { task = "release:source-consumer" },
  { task = "check:prototypes" },
]
`;
    const failures = alignmentFailures(alignReleaseGates(releaseToml, subset));
    expect(failures.length).toBeGreaterThan(0);
    expect(failures.join("\n")).toContain("index 0");
  });

  test("live catalog matches when aligned, fails a deliberate mismatch", async () => {
    const live = checkReleaseGatesAlignment();
    expect(live.actual).toEqual(live.expected);

    scratch ??= await releaseFixtureScratch();
    const root = await scratch.create("longhorn-release-gates-");
    await mkdir(join(root, "config"), { recursive: true });
    await writeFile(join(root, "config/release.toml"), releaseToml);
    await writeFile(
      join(root, "effigy.toml"),
      `[tasks]\n"release:gates" = [{ task = "release:floor" }]\n`,
    );
    expect(() => checkReleaseGatesAlignment(root)).toThrow(/minus workspace/);
  });
});
