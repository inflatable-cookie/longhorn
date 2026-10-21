import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import {
  discoverPrototypeWorkspaceDependencies,
  prototypeCoverageFailures,
  prototypeGroupMemberCoverage,
} from "./prototype-dependency-coverage.ts";

const repoRoot = resolve(import.meta.dir, "..");

test("prototype QA package coverage matches the resolved manifests and detects a removed entry", async () => {
  const [expected, effigyToml] = await Promise.all([
    discoverPrototypeWorkspaceDependencies(repoRoot),
    readFile(resolve(repoRoot, "effigy.toml"), "utf8"),
  ]);
  const prototypeCheckCoverage = prototypeGroupMemberCoverage(effigyToml, "prototype-check");
  const rustdocCoverage = prototypeGroupMemberCoverage(effigyToml, "rustdoc");

  expect(expected).toContain("longhorn-config");
  expect(expected).toContain("longhorn-storage-layout");
  expect(prototypeCoverageFailures(expected, prototypeCheckCoverage)).toEqual([]);
  expect(prototypeCoverageFailures(expected, rustdocCoverage)).toEqual([]);

  const drifted = prototypeCheckCoverage.filter((name) => name !== expected[0]);
  expect(prototypeCoverageFailures(expected, drifted)).toContain(`missing cargo-package:${expected[0]}`);
});
