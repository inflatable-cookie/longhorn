#!/usr/bin/env bun
// Copy skills/agent-control/ into a consumer git repo's
// .claude/skills/agent-control/. Operator-invoked, never automatic.
//
// Usage:
//   bun scripts/install-agent-control-skill.ts --repo <git-repo>
//   bun scripts/install-agent-control-skill.ts <git-repo>

import { cp, lstat, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const sourceDir = join(repoRoot, "skills", "agent-control");
const destRel = join(".claude", "skills", "agent-control");

export function parseInstallTarget(argv: string[]): string | null {
  const args = argv.filter((arg) => arg !== "--");
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--repo") {
      return args[index + 1] ?? null;
    }
    if (arg === "--help" || arg === "-h") {
      throw new Error(usage());
    }
    if (arg.startsWith("-")) {
      throw new Error(`unknown argument ${arg}\n${usage()}`);
    }
    return arg;
  }
  return null;
}

export function usage(): string {
  return [
    "usage: bun scripts/install-agent-control-skill.ts <git-repo>",
    "   or: bun scripts/install-agent-control-skill.ts --repo <git-repo>",
    "The path is the install target. Do not use the global --repo flag",
    "with this task: that switches catalogs and the consumer does not",
    "define it.",
  ].join("\n");
}

export function parseVersionStamp(skill: string): string | null {
  const match = skill.match(/^longhorn_version:\s*"?([0-9]+(?:\.[0-9]+)*)"?\s*$/m);
  return match?.[1] ?? null;
}

export function compareVersions(left: string, right: string): number {
  const leftParts = left.split(".").map((part) => Number.parseInt(part, 10));
  const rightParts = right.split(".").map((part) => Number.parseInt(part, 10));
  const length = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < length; index += 1) {
    const delta = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (delta !== 0) return delta < 0 ? -1 : 1;
  }
  return 0;
}

async function isGitRepo(path: string): Promise<boolean> {
  try {
    await lstat(join(path, ".git"));
    return true;
  } catch {
    return false;
  }
}

export async function installSkill(targetRepo: string): Promise<string> {
  const target = resolve(targetRepo);
  if (!(await isGitRepo(target))) {
    throw new Error(`refusing ${target}: not a git repository`);
  }
  const dest = join(target, destRel);
  const sourceSkill = await readFile(join(sourceDir, "SKILL.md"), "utf8");
  const sourceVersion = parseVersionStamp(sourceSkill);
  if (!sourceVersion) {
    throw new Error("source skill is missing a longhorn_version stamp");
  }

  let destVersion: string | null = null;
  try {
    destVersion = parseVersionStamp(await readFile(join(dest, "SKILL.md"), "utf8"));
  } catch {
    destVersion = null;
  }

  if (destVersion !== null) {
    const cmp = compareVersions(destVersion, sourceVersion);
    if (cmp > 0) {
      throw new Error(
        `refusing to overwrite ${destRel} at ${destVersion} with older ${sourceVersion}`,
      );
    }
    if (cmp === 0) {
      return `already installed ${sourceVersion} at ${destRel} — no-op`;
    }
  }

  await rm(dest, { recursive: true, force: true });
  await cp(sourceDir, dest, { recursive: true });
  if (destVersion === null) {
    return `installed ${sourceVersion} into ${destRel}`;
  }
  return `upgraded ${destRel} from ${destVersion} to ${sourceVersion}`;
}

export async function main(argv = process.argv.slice(2)): Promise<number> {
  let target: string | null;
  try {
    target = parseInstallTarget(argv);
  } catch (error) {
    console.error((error as Error).message);
    return 2;
  }
  if (!target) {
    console.error(usage());
    return 2;
  }
  try {
    console.log(await installSkill(target));
    return 0;
  } catch (error) {
    console.error((error as Error).message);
    return 1;
  }
}

if (import.meta.main) {
  process.exit(await main());
}
