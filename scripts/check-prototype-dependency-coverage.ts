import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import {
  discoverPrototypeWorkspaceDependencies,
  prototypeCoverageFailures,
  prototypeGroupMemberCoverage,
} from "./prototype-dependency-coverage.ts";

export async function checkPrototypeDependencyCoverage(repoRoot: string): Promise<{
  packages: string[];
  prototypeCheckCoverage: string[];
  rustdocCoverage: string[];
}> {
  const [packages, effigyToml] = await Promise.all([
    discoverPrototypeWorkspaceDependencies(repoRoot),
    readFile(resolve(repoRoot, "effigy.toml"), "utf8"),
  ]);
  const prototypeCheckCoverage = prototypeGroupMemberCoverage(effigyToml, "prototype-check");
  const rustdocCoverage = prototypeGroupMemberCoverage(effigyToml, "rustdoc");
  const failures = [
    ...prototypeCoverageFailures(packages, prototypeCheckCoverage).map((failure) => `prototype-check: ${failure}`),
    ...prototypeCoverageFailures(packages, rustdocCoverage).map((failure) => `rustdoc: ${failure}`),
  ];
  if (failures.length > 0) {
    throw new Error(`prototype-dependencies QA group drifted from prototype manifests:\n${failures.map((failure) => `  ${failure}`).join("\n")}`);
  }
  return { packages, prototypeCheckCoverage, rustdocCoverage };
}

if (import.meta.main) {
  const repoRoot = resolve(import.meta.dir, "..");
  const result = await checkPrototypeDependencyCoverage(repoRoot);
  console.log(JSON.stringify({
    schema: "longhorn.prototype-dependency-coverage.v1",
    outcome: "pass",
    packages: result.packages,
    count: result.packages.length,
  }, null, 2));
}
