// Refresh workspace-excluded prototype Cargo.lock files (and the root lock)
// after a Longhorn version bump. Effigy `release prepare` syncs only the root
// `Cargo.lock`; `check:prototypes --locked` then fails because the eight
// prototype workspaces still pin `longhorn-*` crates at the previous version.
//
// `cargo update` re-resolves third-party crates, so this step rewrites only
// path-package version fields to match `workspace.package.version`, then
// proves each lock with `cargo metadata --locked --offline`. It is a pre-gate
// maintenance step: it does not weaken `--locked` and does not belong in
// `[release.gates]`. `effigy release:bump` runs it.

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

import { parseWorkspacePackageVersion } from "./longhorn-version.ts";

export type LockPackage = {
  name: string;
  version: string;
  source: string | undefined;
  body: string;
};

export type ParsedLock = {
  header: string;
  packages: LockPackage[];
};

export type LockAssessment = {
  bumped: string[];
  thirdPartyMoved: string[];
  unexpected: string[];
};

export type LockSyncResult = {
  lock: string;
  stale: string[];
  bumped: string[];
};

export function parseLock(text: string): ParsedLock {
  const split = text.split(/^\[\[package\]\]\n/m);
  const header = split[0] ?? "";
  const packages = split.slice(1).map((body) => {
    const name = /^name = "([^"]+)"/m.exec(body)?.[1];
    const version = /^version = "([^"]+)"/m.exec(body)?.[1];
    const source = /^source = "([^"]+)"/m.exec(body)?.[1];
    if (!name || !version) throw new Error("Cargo.lock package is missing name or version");
    return { name, version, source, body };
  });
  return { header, packages };
}

export function isLonghornPathPackage(pkg: LockPackage): boolean {
  return pkg.name.startsWith("longhorn-") && pkg.source === undefined && pkg.version !== "0.0.0";
}

export function staleLonghornPackages(packages: LockPackage[], version: string): LockPackage[] {
  return packages.filter((pkg) => isLonghornPathPackage(pkg) && pkg.version !== version);
}

export function rewriteLonghornPathVersions(text: string, version: string): string {
  const split = text.split(/^\[\[package\]\]\n/m);
  const header = split[0] ?? "";
  const packages = split.slice(1).map((body) => {
    const name = /^name = "([^"]+)"/m.exec(body)?.[1];
    const source = /^source = "([^"]+)"/m.exec(body)?.[1];
    const current = /^version = "([^"]+)"/m.exec(body)?.[1];
    if (!name || !current) return body;
    if (!name.startsWith("longhorn-") || source !== undefined || current === "0.0.0" || current === version) {
      return body;
    }
    return body.replace(/^version = "[^"]+"$/m, `version = "${version}"`);
  });
  return [header, ...packages.map((body) => `[[package]]\n${body}`)].join("");
}

function packageKey(pkg: LockPackage): string {
  return pkg.source === undefined ? `path:${pkg.name}` : `reg:${pkg.name}@${pkg.version}\0${pkg.source}`;
}

export function assessLock(
  before: ParsedLock,
  after: ParsedLock,
  version: string,
): LockAssessment {
  const unexpected: string[] = [];
  const thirdPartyMoved: string[] = [];
  const bumped: string[] = [];
  if (before.header !== after.header) unexpected.push("lockfile header changed");

  const beforeMap = new Map(before.packages.map((pkg) => [packageKey(pkg), pkg]));
  const afterMap = new Map(after.packages.map((pkg) => [packageKey(pkg), pkg]));
  const beforePath = new Map(
    before.packages.filter((pkg) => pkg.source === undefined).map((pkg) => [pkg.name, pkg]),
  );
  const afterPath = new Map(
    after.packages.filter((pkg) => pkg.source === undefined).map((pkg) => [pkg.name, pkg]),
  );

  for (const name of new Set([...beforePath.keys(), ...afterPath.keys()])) {
    const previous = beforePath.get(name);
    const next = afterPath.get(name);
    if (!previous || !next) {
      unexpected.push(`path package ${name} added or removed`);
      continue;
    }
    const previousBody = previous.body.replace(/^version = "[^"]+"$/m, 'version = "*"');
    const nextBody = next.body.replace(/^version = "[^"]+"$/m, 'version = "*"');
    if (previousBody !== nextBody) {
      unexpected.push(`path package ${name} changed fields other than version`);
      continue;
    }
    if (previous.version === next.version) continue;
    if (isLonghornPathPackage(previous) && next.version === version) {
      bumped.push(`${name} ${previous.version} -> ${next.version}`);
      continue;
    }
    thirdPartyMoved.push(`${name} ${previous.version} -> ${next.version}`);
  }

  for (const key of new Set([...beforeMap.keys(), ...afterMap.keys()])) {
    if (!key.startsWith("reg:")) continue;
    if (beforeMap.has(key) && afterMap.has(key) && beforeMap.get(key)!.body === afterMap.get(key)!.body) {
      continue;
    }
    thirdPartyMoved.push(`registry entry moved: ${key.slice(4)}`);
  }

  return { bumped, thirdPartyMoved, unexpected };
}

export async function cargoMetadataLocked(cwd: string, repoRoot: string): Promise<void> {
  const command = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--no-deps"];
  const process = Bun.spawn(command, { cwd, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed in ${relative(repoRoot, cwd)}\n${stdout}\n${stderr}`);
  }
}

export async function syncPrototypeLocks(
  repoRoot: string,
  options: { verifyMetadata?: boolean } = {},
): Promise<{ workspaceVersion: string; locks: LockSyncResult[] }> {
  const verifyMetadata = options.verifyMetadata ?? true;
  const workspaceVersion = parseWorkspacePackageVersion(
    await readFile(join(repoRoot, "Cargo.toml"), "utf8"),
  );

  const prototypeDirs = (await readdir(join(repoRoot, "prototypes"), { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(repoRoot, "prototypes", entry.name))
    .sort();
  if (prototypeDirs.length === 0) throw new Error("no prototypes/* directories");

  const lockTargets = [
    { dir: repoRoot, lock: join(repoRoot, "Cargo.lock") },
    ...prototypeDirs.map((dir) => ({ dir, lock: join(dir, "Cargo.lock") })),
  ];

  const results: LockSyncResult[] = [];
  for (const target of lockTargets) {
    const original = await readFile(target.lock, "utf8");
    const before = parseLock(original);
    const stale = staleLonghornPackages(before.packages, workspaceVersion);
    const nextText = stale.length === 0 ? original : rewriteLonghornPathVersions(original, workspaceVersion);
    if (nextText !== original) await writeFile(target.lock, nextText);
    const after = parseLock(await readFile(target.lock, "utf8"));
    const assessment = assessLock(before, after, workspaceVersion);
    if (assessment.thirdPartyMoved.length > 0 || assessment.unexpected.length > 0) {
      throw new Error(
        `${relative(repoRoot, target.lock)} moved more than Longhorn path versions:\n` +
          assessment.thirdPartyMoved.concat(assessment.unexpected).map((line) => `  ${line}`).join("\n"),
      );
    }
    if (verifyMetadata) await cargoMetadataLocked(target.dir, repoRoot);
    results.push({
      lock: relative(repoRoot, target.lock),
      stale: stale.map((pkg) => pkg.name),
      bumped: assessment.bumped,
    });
  }

  return { workspaceVersion, locks: results };
}

if (import.meta.main) {
  const repoRoot = resolve(import.meta.dir, "..");
  const result = await syncPrototypeLocks(repoRoot);
  console.log(JSON.stringify({
    schema: "longhorn.prototype-lock-sync.v1",
    outcome: "pass",
    workspaceVersion: result.workspaceVersion,
    locks: result.locks,
  }, null, 2));
}
