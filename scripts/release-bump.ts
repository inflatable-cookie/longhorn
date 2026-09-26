#!/usr/bin/env bun
// Coordinated Longhorn version bump. One command updates the workspace
// version, internal pins, npm versions and adapter peers, the agent-control
// skill stamp, the changelog heading, excluded prototype locks, and the
// generated API reference. It reports every file it changed, including lock
// rewrites. It refuses a non-increasing version, is idempotent at the same
// version, and fails if a lock line other than a Longhorn path-package
// version moved. A failure after any write restores the files it touched.

import { readdir, readFile, unlink, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import { compareSemver, parseSemver, parseWorkspacePackageVersion } from "./longhorn-version.ts";
import { syncPrototypeLocks } from "./sync-prototype-locks.ts";

const defaultRepoRoot = resolve(import.meta.dir, "..");
const LONGHORN_PEER = "@inflatable-cookie/longhorn";

export type ReleaseBumpOptions = {
  repoRoot?: string;
  version: string;
  date?: string;
  syncLocks?: (repoRoot: string) => Promise<unknown>;
  regenerateApiReference?: (repoRoot: string) => Promise<void>;
};

export type ReleaseBumpResult = {
  readonly previousVersion: string;
  readonly version: string;
  readonly changed: readonly string[];
  readonly idempotent: boolean;
};

export function todayStamp(now = new Date()): string {
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function replaceWorkspacePackageVersion(manifest: string, version: string): string {
  const match = /\[workspace\.package\][^[]*/m.exec(manifest);
  if (!match) throw new Error("Cargo.toml has no [workspace.package] table");
  const updated = match[0].replace(/^version = "[^"]+"/m, `version = "${version}"`);
  if (updated === match[0] && !match[0].includes(`version = "${version}"`)) {
    throw new Error("workspace.package.version was not replaced");
  }
  return `${manifest.slice(0, match.index)}${updated}${manifest.slice(match.index + match[0].length)}`;
}

export function replaceLonghornPathPins(manifest: string, version: string): string {
  return manifest.replace(
    /^(longhorn-[A-Za-z0-9-]+ = \{ path = "[^"]+", version = ")[^"]+(".*)$/gm,
    `$1${version}$2`,
  );
}

export function bumpPackageManifest(text: string, version: string): string {
  const versionLine = /^  "version": "[^"]+",/m;
  if (!versionLine.test(text)) {
    throw new Error('package.json version line must be of the form `  "version": "...",`');
  }
  const next = text.replace(versionLine, `  "version": "${version}",`);
  return next.replace(
    new RegExp(`("${LONGHORN_PEER.replace("/", "\\/")}": ")[^"]+(")`, "g"),
    `$1${version}$2`,
  );
}

export function bumpSkillStamp(skill: string, version: string): string {
  if (!/^longhorn_version:/m.test(skill)) {
    throw new Error("skills/agent-control/SKILL.md is missing a longhorn_version stamp");
  }
  return skill.replace(/^longhorn_version:\s*.*$/m, `longhorn_version: "${version}"`);
}

export function promoteChangelog(text: string, version: string, date: string): string {
  if (text.includes(`## [${version}]`)) return text;
  const match = /^(## \[Unreleased\]\n)([\s\S]*?)(?=^## \[)/m.exec(text);
  if (!match) throw new Error("CHANGELOG.md is missing a parseable ## [Unreleased] section");
  const body = match[2] ?? "";
  const notes = body.trim();
  const heading = `## [${version}] - ${date}`;
  const promoted = notes.length > 0
    ? `## [Unreleased]\n\n${heading}\n\n${notes}\n\n`
    : `## [Unreleased]\n\n${heading}\n\n`;
  return `${text.slice(0, match.index)}${promoted}${text.slice(match.index + match[0].length)}`;
}

async function defaultRegenerateApiReference(repoRoot: string): Promise<void> {
  const command = ["bun", join(repoRoot, "scripts/generate-api-reference-card126.ts"), "--write"];
  const process = Bun.spawn(command, { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
  }
}

async function writeIfChanged(path: string, next: string): Promise<boolean> {
  const previous = await readFile(path, "utf8");
  if (previous === next) return false;
  await writeFile(path, next);
  return true;
}

type SnapshottedFile = {
  relative: string;
  path: string;
  content: string | null;
};

async function lockRelatives(repoRoot: string): Promise<string[]> {
  const prototypeLocks: string[] = [];
  const entries = await readdir(join(repoRoot, "prototypes"), { withFileTypes: true }).catch(() => []);
  for (const entry of entries) {
    if (entry.isDirectory()) prototypeLocks.push(`prototypes/${entry.name}/Cargo.lock`);
  }
  prototypeLocks.sort();
  return ["Cargo.lock", ...prototypeLocks];
}

async function snapshotFiles(repoRoot: string, relatives: string[]): Promise<SnapshottedFile[]> {
  const unique = [...new Set(relatives)];
  return Promise.all(unique.map(async (relative) => {
    const path = join(repoRoot, relative);
    const content = await readFile(path, "utf8").catch(() => null);
    return { relative, path, content };
  }));
}

async function restoreFiles(snapshots: SnapshottedFile[]): Promise<void> {
  for (const file of snapshots) {
    if (file.content === null) {
      await unlink(file.path).catch(() => undefined);
    } else {
      await writeFile(file.path, file.content);
    }
  }
}

export async function bumpRelease(options: ReleaseBumpOptions): Promise<ReleaseBumpResult> {
  parseSemver(options.version);
  const repoRoot = options.repoRoot ?? defaultRepoRoot;
  const version = options.version;
  const date = options.date ?? todayStamp();
  const cargoPath = join(repoRoot, "Cargo.toml");
  const previousVersion = parseWorkspacePackageVersion(await readFile(cargoPath, "utf8"));
  if (compareSemver(version, previousVersion) < 0) {
    throw new Error(`refusing non-increasing version ${version} (current ${previousVersion})`);
  }

  const planned = new Map<string, string>();
  planned.set(
    "Cargo.toml",
    replaceLonghornPathPins(
      replaceWorkspacePackageVersion(await readFile(cargoPath, "utf8"), version),
      version,
    ),
  );

  const packageDirs = (await readdir(join(repoRoot, "packages"), { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  for (const directory of packageDirs) {
    const relative = `packages/${directory}/package.json`;
    const text = await readFile(join(repoRoot, relative), "utf8").catch(() => null);
    if (text === null) continue;
    try {
      planned.set(relative, bumpPackageManifest(text, version));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`${relative}: ${message}`);
    }
  }

  planned.set(
    "skills/agent-control/SKILL.md",
    bumpSkillStamp(await readFile(join(repoRoot, "skills/agent-control/SKILL.md"), "utf8"), version),
  );
  planned.set(
    "CHANGELOG.md",
    promoteChangelog(await readFile(join(repoRoot, "CHANGELOG.md"), "utf8"), version, date),
  );

  const snapshots = await snapshotFiles(repoRoot, [
    ...planned.keys(),
    ...(await lockRelatives(repoRoot)),
    "docs/reference/api-surface.md",
  ]);

  try {
    for (const [relative, next] of planned) {
      await writeIfChanged(join(repoRoot, relative), next);
    }

    const syncLocks = options.syncLocks ?? ((root: string) => syncPrototypeLocks(root));
    await syncLocks(repoRoot);

    const regenerate = options.regenerateApiReference ?? defaultRegenerateApiReference;
    await regenerate(repoRoot);

    const changed: string[] = [];
    for (const file of snapshots) {
      const next = await readFile(file.path, "utf8").catch(() => null);
      if (next !== file.content) changed.push(file.relative);
    }

    return {
      previousVersion,
      version,
      changed,
      idempotent: compareSemver(version, previousVersion) === 0 && changed.length === 0,
    };
  } catch (error) {
    try {
      await restoreFiles(snapshots);
    } catch (restoreError) {
      const first = error instanceof Error ? error.message : String(error);
      const second = restoreError instanceof Error ? restoreError.message : String(restoreError);
      throw new Error(`release bump failed (${first}); restore failed (${second})`);
    }
    throw error;
  }
}

if (import.meta.main) {
  const version = process.argv.find((arg) => /^\d+\.\d+\.\d+$/.test(arg));
  if (!version) {
    throw new Error("usage: bun scripts/release-bump.ts <X.Y.Z>");
  }
  const result = await bumpRelease({ version });
  console.log(JSON.stringify({
    schema: "longhorn.release-bump.v1",
    outcome: "pass",
    ...result,
  }, null, 2));
}
