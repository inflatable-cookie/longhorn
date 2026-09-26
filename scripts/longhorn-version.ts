// Single source for the coordinated Longhorn version: Cargo.toml
// `[workspace.package].version`. Proofs, boundary tests, bump, and lock sync
// read this instead of literals. Historical logs, evidence, and changelog
// entries keep their own version strings.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const defaultRepoRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));

export function parseWorkspacePackageVersion(manifest: string): string {
  const match = /\[workspace\.package\][^[]*?^version = "([^"]+)"/m.exec(manifest);
  if (!match?.[1]) {
    throw new Error("workspace.package.version not found in Cargo.toml");
  }
  return match[1];
}

export function readWorkspacePackageVersion(repoRoot = defaultRepoRoot): string {
  return parseWorkspacePackageVersion(readFileSync(join(repoRoot, "Cargo.toml"), "utf8"));
}

export function longhornVersion(repoRoot = defaultRepoRoot): string {
  return readWorkspacePackageVersion(repoRoot);
}

export function parseSemver(version: string): [number, number, number] {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) {
    throw new Error(`expected X.Y.Z semver, got ${version}`);
  }
  return [
    Number.parseInt(match[1], 10),
    Number.parseInt(match[2], 10),
    Number.parseInt(match[3], 10),
  ];
}

export function compareSemver(left: string, right: string): number {
  const leftParts = parseSemver(left);
  const rightParts = parseSemver(right);
  for (let index = 0; index < 3; index += 1) {
    const delta = leftParts[index]! - rightParts[index]!;
    if (delta !== 0) return delta < 0 ? -1 : 1;
  }
  return 0;
}
