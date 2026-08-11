import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { ArtifactIdentity, FrozenSource } from "./types";

export const repoRoot = resolve(import.meta.dir, "../..");
// No sibling default: a path above the repository root is exactly what made
// local gates lie. Card 125 greenfield is the only admitted escape.
export const poodleRoot = resolve(
  process.env.POODLE_REPO ||
    (() => {
      throw new Error(
        "POODLE_REPO must point at a Poodle checkout; sibling defaults are forbidden",
      );
    })(),
);
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

export async function run(command: string[], cwd = repoRoot): Promise<string> {
  const process = Bun.spawn(command, { cwd, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) throw new Error(`${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`);
  return stdout;
}

export async function git(root: string, args: string[]): Promise<string> {
  return (await run(["git", ...args], root)).trim();
}

export async function digest(path: string): Promise<string> {
  return hash(await readFile(path));
}

export function hash(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

export function artifactSet(identities: Array<ArtifactIdentity | (ArtifactIdentity & { inventorySha256: string })>): string {
  return hash(identities
    .map((identity) => `${identity.name}:${identity.sha256}:${"inventorySha256" in identity ? identity.inventorySha256 : ""}`)
    .sort()
    .join("\n"));
}

export function fileDependency(path: string): string {
  return `file:${path}`;
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
