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

function readWorkspacePackageVersion(repoRoot = defaultRepoRoot): string {
  return parseWorkspacePackageVersion(readFileSync(join(repoRoot, "Cargo.toml"), "utf8"));
}

export function longhornVersion(repoRoot = defaultRepoRoot): string {
  return readWorkspacePackageVersion(repoRoot);
}

export function parseSemver(version: string): [number, number, number] {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match?.[1] || !match[2] || !match[3]) {
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
  const delta =
    leftParts[0] - rightParts[0] || leftParts[1] - rightParts[1] || leftParts[2] - rightParts[2];
  return Math.sign(delta);
}
