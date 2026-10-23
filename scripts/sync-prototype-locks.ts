// Refresh workspace-excluded prototype Cargo.lock files (and the root lock).
// `release prepare` syncs only the root lock; this selector also admits a
// missing Longhorn path package when an updated workspace graph requires it.
//
// Missing Longhorn path-package entries are copied from the root workspace
// lock with their dependency edges. The lock diff is restricted to these
// additions and Longhorn version rewrites. Third-party movement is rejected;
// `cargo metadata --locked --offline --no-deps` proves each resulting lock. The
// call runs through `scripts/rust-command.sh`, the one Cargo switch point.

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

import { parseWorkspacePackageVersion } from "./longhorn-version.ts";
import { rustCommand } from "./rust-command.ts";

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
  added: string[];
  bumped: string[];
  thirdPartyMoved: string[];
  unexpected: string[];
};

export type LockSyncResult = {
  lock: string;
  added: string[];
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

export function dependencyReferences(pkg: LockPackage): { name: string; version: string | undefined }[] {
  const dependencies = /^dependencies = \[([\s\S]*?)^\]/m.exec(pkg.body)?.[1];
  if (!dependencies) return [];
  return [...dependencies.matchAll(/"([^"]+)"/g)].map(([, reference]) => {
    const [name, version] = reference!.split(/\s+/, 2);
    return { name: name!, version };
  });
}

function packageMatchesReference(pkg: LockPackage, reference: { name: string; version: string | undefined }): boolean {
  return pkg.name === reference.name && (reference.version === undefined || pkg.version === reference.version);
}

function serializeLock(lock: ParsedLock): string {
  const sorted = [...lock.packages].sort((left, right) =>
    left.name.localeCompare(right.name) || left.version.localeCompare(right.version),
  );
  return [lock.header, ...sorted.map((pkg) => `[[package]]\n${pkg.body}`)].join("");
}

export function addMissingLonghornPathPackages(
  prototypeLockText: string,
  workspaceLockText: string,
): { text: string; added: string[] } {
  const original = parseLock(prototypeLockText);
  const workspacePackages = parseLock(workspaceLockText).packages.filter(isLonghornPathPackage);
  const packages = [...original.packages];
  const added = new Set<string>();

  while (true) {
    const missingLonghorn = new Map<string, { name: string; version: string | undefined }>();
    for (const pkg of packages) {
      for (const reference of dependencyReferences(pkg)) {
        if (packages.some((candidate) => packageMatchesReference(candidate, reference))) continue;
        if (reference.name.startsWith("longhorn-")) missingLonghorn.set(reference.name, reference);
      }
    }
    if (missingLonghorn.size === 0) break;

    let inserted = false;
    for (const reference of missingLonghorn.values()) {
      const workspacePackage = workspacePackages.find((pkg) => packageMatchesReference(pkg, reference));
      if (!workspacePackage) {
        throw new Error(`prototype lock references missing Longhorn path package ${reference.name}, absent from the workspace lock`);
      }
      packages.push(workspacePackage);
      added.add(workspacePackage.name);
      inserted = true;
    }
    if (!inserted) break;
  }

  // A missing registry package is never repairable from the workspace lock.
  // Stop before any Cargo command can select a new version.
  for (const pkg of packages) {
    for (const reference of dependencyReferences(pkg)) {
      if (packages.some((candidate) => packageMatchesReference(candidate, reference))) continue;
      if (!reference.name.startsWith("longhorn-")) {
        throw new Error(`prototype lock is missing third-party package ${reference.name}; refusing to resolve it`);
      }
    }
  }

  return {
    text: added.size === 0 ? prototypeLockText : serializeLock({ header: original.header, packages }),
    added: [...added].sort(),
  };
}

function packageKey(pkg: LockPackage): string {
  return pkg.source === undefined ? `path:${pkg.name}` : `reg:${pkg.name}@${pkg.version}\0${pkg.source}`;
}

export function assessLock(
  before: ParsedLock,
  after: ParsedLock,
  version: string,
  allowedPathPackages: LockPackage[] = [],
): LockAssessment {
  const allowedPathNames = new Set(
    allowedPathPackages.filter(isLonghornPathPackage).map((pkg) => pkg.name),
  );
  const unexpected: string[] = [];
  const thirdPartyMoved: string[] = [];
  const added: string[] = [];
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
    if (!previous && next && allowedPathNames.has(name) && isLonghornPathPackage(next) && next.version === version) {
      added.push(name);
      continue;
    }
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

  return { added, bumped, thirdPartyMoved, unexpected };
}

export async function cargoMetadataLocked(cwd: string, repoRoot: string): Promise<void> {
  const command = rustCommand(["metadata", "--format-version", "1", "--locked", "--offline", "--no-deps"]);
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

  const originals = new Map<string, string>();
  const candidates = new Map<string, string>();
  const results: LockSyncResult[] = [];
  try {
    const originalWorkspaceLock = await readFile(join(repoRoot, "Cargo.lock"), "utf8");
    const workspaceLock = rewriteLonghornPathVersions(originalWorkspaceLock, workspaceVersion);
    const allowedPathPackages = parseLock(workspaceLock).packages.filter(isLonghornPathPackage);

    for (const target of lockTargets) {
      const original = target.dir === repoRoot ? originalWorkspaceLock : await readFile(target.lock, "utf8");
      originals.set(target.lock, original);
      const before = parseLock(original);
      const seeded = target.dir === repoRoot
        ? { text: workspaceLock, added: [] as string[] }
        : addMissingLonghornPathPackages(original, workspaceLock);
      const candidate = rewriteLonghornPathVersions(seeded.text, workspaceVersion);
      const after = parseLock(candidate);
      const assessment = assessLock(before, after, workspaceVersion, allowedPathPackages);
      if (assessment.thirdPartyMoved.length > 0 || assessment.unexpected.length > 0) {
        throw new Error(
          `${relative(repoRoot, target.lock)} moved more than Longhorn path packages:\n` +
            assessment.thirdPartyMoved.concat(assessment.unexpected).map((line) => `  ${line}`).join("\n"),
        );
      }
      candidates.set(target.lock, candidate);
      results.push({
        lock: relative(repoRoot, target.lock),
        added: assessment.added,
        stale: staleLonghornPackages(before.packages, workspaceVersion).map((pkg) => pkg.name),
        bumped: assessment.bumped,
      });
    }

    for (const [lock, candidate] of candidates) await writeFile(lock, candidate);
    if (verifyMetadata) {
      for (const target of lockTargets) await cargoMetadataLocked(target.dir, repoRoot);
    }
  } catch (error) {
    for (const [lock, original] of originals) await writeFile(lock, original);
    throw error;
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
