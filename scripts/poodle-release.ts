// One source of truth for which published Poodle release the proofs run
// against.
//
// This replaces `poodle-evidence.ts`, which pinned the proofs to a packed
// tarball in a sibling checkout. That indirection existed because Poodle was
// unpublished: the only way to prove a consumer could install it was to pack it
// and install the pack. Poodle 0.1.0 is on the public registry, so the proofs
// install what a real consumer installs, and the pack directory is gone.
//
// The integrity claim survives the move rather than being dropped. It used to
// be a SHA-256 over each pack on disk plus a membership hash over the set. It
// is now the sha512 subresource integrity bun records in `bun.lock` for each
// published package, checked against the version the root manifest asks for and
// against the copy actually installed in `node_modules`. Both forms answer the
// same question — are the proofs verifying the bytes a consumer resolves — and
// the registry form is stronger, because a consumer can verify it too.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

const CORE = "@inflatable-cookie/poodle-core";
const SVELTE = "@inflatable-cookie/poodle-svelte";

export interface PoodlePackage {
  readonly name: string;
  readonly version: string;
  readonly integrity: string;
}

export interface PoodleRelease {
  /** The version every published Poodle package in the closure carries. */
  readonly version: string;
  readonly packages: readonly PoodlePackage[];
}

function rootManifest(): {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  overrides?: Record<string, string>;
} {
  return JSON.parse(readFileSync(resolve(repoRoot, "package.json"), "utf8"));
}

function pinnedRange(): string {
  const manifest = rootManifest();
  const pin =
    manifest.devDependencies?.[CORE] ??
    manifest.dependencies?.[CORE] ??
    manifest.overrides?.[CORE];
  if (!pin) {
    throw new Error(`${CORE} is not a dependency of the root manifest`);
  }
  if (pin.startsWith("file:") || pin.startsWith("link:")) {
    throw new Error(
      `${CORE} is pinned to ${pin}. The proofs verify the published release, ` +
        "so a path reference means they would be proving something no consumer resolves.",
    );
  }
  return pin;
}

/**
 * Caret satisfaction, only as far as this repository needs it. Every Poodle and
 * Longhorn package carries one version in lockstep, and all of them are below
 * 1.0, where a caret is pinned to the minor. Anything more general would be a
 * semver implementation nobody asked for.
 */
function satisfiesCaret(version: string, range: string): boolean {
  if (!range.startsWith("^")) return version === range;
  const want = range.slice(1).split(".").map(Number);
  const have = version.split(".").map(Number);
  if (want.length !== 3 || have.length !== 3 || [...want, ...have].some(Number.isNaN)) {
    return false;
  }
  if (want[0] !== have[0]) return false;
  if (want[0] === 0 && want[1] !== have[1]) return false;
  if (have[1] < want[1]) return false;
  if (have[1] === want[1] && have[2] < want[2]) return false;
  return true;
}

/**
 * bun.lock is JSONC, and there is no JSONC parser to hand. The entry shape is
 * stable and narrow, so match it directly and fail loudly rather than pull in a
 * parser to read two lines.
 *
 *   "<name>": ["<name>@<version>", "<registry>", { … }, "sha512-…"],
 */
function lockedPackage(name: string): PoodlePackage {
  const lock = readFileSync(resolve(repoRoot, "bun.lock"), "utf8");
  const escaped = name.replace(/[/@-]/g, (character) => `\\${character}`);
  const entry = new RegExp(
    `"${escaped}":\\s*\\["${escaped}@([^"]+)"[\\s\\S]*?"(sha512-[^"]+)"\\s*\\]`,
  ).exec(lock);
  if (!entry) {
    throw new Error(
      `bun.lock records no registry entry for ${name}. ` +
        "Run `bun install` after repointing, or the package is still resolved from a path.",
    );
  }
  return { name, version: entry[1], integrity: entry[2] };
}

function installedVersion(name: string): string {
  const manifest = resolve(repoRoot, "node_modules", name, "package.json");
  try {
    return JSON.parse(readFileSync(manifest, "utf8")).version as string;
  } catch {
    throw new Error(`${name} is not installed; run \`bun install\``);
  }
}

let cached: PoodleRelease | null = null;

export function poodleRelease(): PoodleRelease {
  if (cached) return cached;

  const range = pinnedRange();
  const packages = [CORE, SVELTE].map(lockedPackage);

  for (const pkg of packages) {
    if (!satisfiesCaret(pkg.version, range)) {
      throw new Error(
        `${pkg.name} is locked at ${pkg.version}, which does not satisfy ${range}`,
      );
    }
    const onDisk = installedVersion(pkg.name);
    if (onDisk !== pkg.version) {
      throw new Error(
        `${pkg.name} is locked at ${pkg.version} but ${onDisk} is installed; the tree and the lockfile disagree`,
      );
    }
  }

  // Lockstep is a claim this repository makes, so it is checked rather than
  // assumed: one version across every published package in the closure.
  const versions = [...new Set(packages.map((pkg) => pkg.version))];
  if (versions.length !== 1) {
    throw new Error(
      `Poodle packages are not in lockstep: ${packages
        .map((pkg) => `${pkg.name}@${pkg.version}`)
        .join(", ")}`,
    );
  }

  cached = { version: versions[0], packages };
  return cached;
}

/** The published Poodle version the proofs run against. */
export function poodleVersion(): string {
  return poodleRelease().version;
}

/** Package names in the pinned release, sorted. */
export function poodlePackageNames(): readonly string[] {
  return poodleRelease()
    .packages.map((pkg) => pkg.name)
    .slice()
    .sort();
}

/**
 * The Svelte peer range the published adapter actually declares. This used to
 * be read from Poodle's own evidence file; reading it from the installed
 * package checks the artifact rather than a claim about the artifact.
 */
export function poodleSveltePeerRange(): string {
  const manifest = resolve(repoRoot, "node_modules", SVELTE, "package.json");
  const peers = JSON.parse(readFileSync(manifest, "utf8")).peerDependencies as
    | Record<string, string>
    | undefined;
  const range = peers?.svelte;
  if (!range) {
    throw new Error(`${SVELTE} declares no svelte peer dependency`);
  }
  return range;
}
