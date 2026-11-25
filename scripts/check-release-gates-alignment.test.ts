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
"release:gates" = [
  "bun scripts/verify-private-candidate-docs.ts",
  "cargo deny check advisories",
  { task = "docs:rust" },
  { task = "check:prototypes" },
  { task = "release:floor" },
  { task = "release:source-consumer" },
]
`;

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
    await writeFile(join(root, "effigy.toml"), `"release:gates" = [{ task = "release:floor" }]\n`);
    expect(() => checkReleaseGatesAlignment(root)).toThrow(/minus workspace/);
  });
});
