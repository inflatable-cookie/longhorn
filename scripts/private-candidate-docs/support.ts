import { run, sha256 } from "../proof-support.ts";
import { resolve } from "node:path";
import type { ArtifactIdentity, FrozenSource } from "./types";

export const repoRoot = resolve(import.meta.dir, "../..");
export const candidateVersion = "0.1.0";

export function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

export function equal<T>(actual: T, expected: T, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
  }
}

export function equalSet(actual: readonly string[], expected: readonly string[], label: string): void {
  equal([...actual].sort(), [...expected].sort(), label);
}

export async function git(root: string, args: string[]): Promise<string> {
  return (await run(["git", ...args], root)).trim();
}

export function artifactSet(identities: Array<ArtifactIdentity | (ArtifactIdentity & { inventorySha256: string })>): string {
  return sha256(identities
    .map((identity) => `${identity.name}:${identity.sha256}:${"inventorySha256" in identity ? identity.inventorySha256 : ""}`)
    .sort()
    .join("\n"));
}

export async function freezeSource(root: string, expectedCommit: string, selectedPaths: string[]): Promise<FrozenSource> {
  equal(await git(root, ["rev-parse", "HEAD"]), expectedCommit, `${root} selected commit`);
  const status = await git(root, ["status", "--porcelain", "--", ...selectedPaths]);
  assert(!status, `${root} selected paths are dirty:\n${status}`);
  const drift = Bun.spawnSync(["git", "diff", "--quiet", expectedCommit, "--", ...selectedPaths], {
    cwd: root,
    stdout: "pipe",
    stderr: "pipe",
  });
  assert(drift.exitCode === 0, `${root} selected paths drift from ${expectedCommit}`);
  return {
    branch: await git(root, ["branch", "--show-current"]),
    commit: expectedCommit,
    selectedPathsClean: true,
    selectedPaths: [...selectedPaths].sort(),
  };
}

export function stableJson(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}
