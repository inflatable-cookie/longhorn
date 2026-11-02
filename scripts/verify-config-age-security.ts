// Proves the repaired age dependency line (RUSTSEC-2026-0387 repair: age
// 0.12.1 -> 0.11.5) survives public consumer resolution at the exact working
// tree.
//
// The root Cargo.lock cannot fix downstream Git consumers: they resolve their
// own graph against Longhorn's manifest requirements. This proof stages a
// disposable Git-dependency consumer of `longhorn-config-age` (which drags
// `longhorn-config` and `longhorn-core`) at the exact tree, then asserts:
//
// 1. unlocked compatible resolution — a fresh lockfile resolves age 0.11.5
//    with age-core 0.11.x and none of the RUSTSEC-2026-0387 transitive chain
//    (crypto-bigint, elliptic-curve, p256, hpke, ml-kem), and every Longhorn
//    package carries a git source at the exact snapshot commit;
// 2. the consumer compiles --locked against that resolution;
// 3. stale downstream lock — a consumer whose lock resolved the vulnerable
//    line against the pre-repair candidate cannot keep it under --locked
//    after repointing at the repaired tree, and one re-lock lands on the
//    repaired line with the chain gone. (A consumer may still pin its own
//    separate age 0.12.x copy as a semver-incompatible duplicate for its own
//    direct use; no manifest requirement can prevent that, and Longhorn's
//    guarantee is about the resolution it participates in.)
//
// Final Cargo outputs for the compile stay in a private directory on external
// build scratch; the staged consumers live under the run's temp root.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";

import {
  prepareStagedWorkspace,
  privateCargoCommand,
  rustExec,
  taskScratch,
} from "./rust-command.ts";

const REPAIR_AGE_VERSION = "0.11.5";
const REPAIR_AGE_CORE_PREFIX = "0.11.";
const VULNERABLE_AGE_VERSION = "0.12.1";
const VULNERABLE_CHAIN = [
  "crypto-bigint",
  "elliptic-curve",
  "hpke",
  "ml-kem",
  "p256",
] as const;
const CONSUMED_CRATES = ["longhorn-config-age", "longhorn-config"] as const;
// The pre-repair candidate whose resolution a real deployed consumer lock
// would carry; the stale-lock probe starts there (task longhorn#086).
const PREREPAIR_COMMIT = "1e1f570c88091b7a9196438bc05e140071babc33";

const repositoryRoot = resolveRepository(import.meta.dir);

function resolveRepository(root: string): string {
  const probe = spawnSync("git", ["rev-parse", "--show-toplevel"], { cwd: root, encoding: "utf8" });
  if (probe.status !== 0) throw new Error(`no git repository above ${root}`);
  return probe.stdout.trim();
}

function run(command: readonly string[], cwd: string): string {
  const result = spawnSync(command[0], command.slice(1), {
    cwd,
    encoding: "utf8",
    env: process.env,
  });
  if (result.status !== 0) {
    throw new Error(
      `command failed (${result.status}): ${command.join(" ")}\nstdout: ${result.stdout}\nstderr: ${result.stderr}`,
    );
  }
  return result.stdout;
}

function attempt(command: readonly string[], cwd: string): { ok: boolean; output: string } {
  const result = spawnSync(command[0], command.slice(1), {
    cwd,
    encoding: "utf8",
    env: process.env,
  });
  return {
    ok: result.status === 0,
    output: `${result.stdout ?? ""}\n${result.stderr ?? ""}`.trim(),
  };
}

function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

function releaseMsrv(): string {
  const env = readFileSafe(join(repositoryRoot, "release-baselines", "rust-toolchains.env"));
  const match = /LONGHORN_GENERAL_MSRV=(\d+)\.(\d+)\.(\d+)/.exec(env ?? "");
  if (!match) throw new Error("release-baselines/rust-toolchains.env has no LONGHORN_GENERAL_MSRV");
  return `${match[1]}.${match[2]}`;
}

function readFileSafe(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

type Snapshot = { root: string; commit: string; kind: "commit" | "snapshot"; temp?: string };

/**
 * Freezes the exact working tree into a Git snapshot a file:// dependency can
 * resolve: clean trees resolve at HEAD; dirty trees copy every tracked and
 * untracked file into a throwaway repository.
 */
async function snapshotWorkingTree(): Promise<Snapshot> {
  const status = run(["git", "status", "--porcelain"], repositoryRoot).trim();
  if (status === "") {
    return {
      root: repositoryRoot,
      commit: run(["git", "rev-parse", "HEAD"], repositoryRoot).trim(),
      kind: "commit",
    };
  }
  const temp = await mkdtemp(join(tmpdir(), "longhorn-config-age-security-source-"));
  const listed = run(
    ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    repositoryRoot,
  );
  for (const relative of listed.split("\0")) {
    if (relative === "") continue;
    const from = join(repositoryRoot, relative);
    const to = join(temp, relative);
    await mkdir(dirname(to), { recursive: true });
    await copyFile(from, to);
  }
  run(["git", "init", "-q"], temp);
  run(["git", "add", "-A"], temp);
  run(
    [
      "git",
      "-c", "user.name=Longhorn",
      "-c", "user.email=config-age-security@invalid",
      "-c", "commit.gpgsign=false",
      "commit", "-q", "-m", "Config age security verification snapshot",
    ],
    temp,
  );
  const env = {
    GIT_AUTHOR_DATE: "2000-01-01T00:00:00Z",
    GIT_COMMITTER_DATE: "2000-01-01T00:00:00Z",
  };
  Object.assign(process.env, env);
  const commit = run(["git", "rev-parse", "HEAD"], temp).trim();
  return { root: temp, commit, kind: "snapshot", temp };
}

function consumerManifest(snapshot: Snapshot): string {
  const url = `file://${snapshot.root}`;
  const dependencies = CONSUMED_CRATES.map(
    (crate) => `${crate} = { git = "${url}", rev = "${snapshot.commit}" }`,
  );
  return `[package]
name = "longhorn-config-age-security-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "${releaseMsrv()}"
publish = false

[workspace]

[dependencies]
${dependencies.join("\n")}
`;
}

const CONSUMER_MAIN = `use longhorn_config::Sha256Digest;
use longhorn_config_age::{AgeIdentity, AGE_V1_FORMAT_ID};

fn main() {
    let identity = AgeIdentity::generate();
    let recipient = identity.recipient().as_string();
    let digest = Sha256Digest::from_bytes(recipient.as_bytes());
    println!(
        "consumer-safe format={AGE_V1_FORMAT_ID} recipient-bytes={} digest={}",
        recipient.len(),
        digest.as_str().len()
    );
}
`;

type MetadataPackage = {
  name: string;
  version: string;
  source: string | null;
};

function assertUnlockedResolution(metadata: unknown, snapshot: Snapshot): void {
  const packages = (metadata as { packages: MetadataPackage[] }).packages;
  const byName = new Map(packages.map((pkg) => [pkg.name, pkg]));

  const longhorn = packages.filter(
    (pkg) => pkg.name.startsWith("longhorn-") && pkg.name !== "longhorn-config-age-security-consumer",
  );
  if (longhorn.length < CONSUMED_CRATES.length + 1) {
    throw new Error(
      `expected at least ${CONSUMED_CRATES.length + 1} Longhorn packages (consumers plus transitive core), got ${longhorn.length}`,
    );
  }
  for (const pkg of longhorn) {
    if (pkg.source === null || !(pkg.source.startsWith("git+file://") && pkg.source.endsWith(`#${snapshot.commit}`))) {
      throw new Error(`${pkg.name} did not resolve from git at the exact snapshot commit: ${pkg.source}`);
    }
  }

  const age = byName.get("age");
  if (age === undefined || age.version !== REPAIR_AGE_VERSION) {
    throw new Error(`age did not resolve to ${REPAIR_AGE_VERSION}: ${age?.version ?? "absent"}`);
  }
  const ageCore = byName.get("age-core");
  if (ageCore === undefined || !ageCore.version.startsWith(REPAIR_AGE_CORE_PREFIX)) {
    throw new Error(`age-core did not resolve on the ${REPAIR_AGE_CORE_PREFIX}x line: ${ageCore?.version ?? "absent"}`);
  }
  for (const banned of VULNERABLE_CHAIN) {
    if (byName.has(banned)) {
      throw new Error(`the RUSTSEC-2026-0387 chain still resolves ${banned}`);
    }
  }
}

async function stageConsumer(
  label: string,
  snapshot: Snapshot,
  extraAgeRequirement?: string,
): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), `longhorn-config-age-security-${label}-`));
  await prepareStagedWorkspace(directory);
  await writeFile(join(directory, "Cargo.toml"), consumerManifest(snapshot, extraAgeRequirement));
  const sourceDirectory = join(directory, "src");
  await mkdir(sourceDirectory);
  await writeFile(join(sourceDirectory, "main.rs"), CONSUMER_MAIN);
  return directory;
}

const snapshot = await snapshotWorkingTree();
const buildScratch = await taskScratch("config-age-security-consumer");
let consumerUnlocked: string | undefined;
let consumerStale: string | undefined;
let receipt: Record<string, unknown>;

try {
  consumerUnlocked = await stageConsumer("unlocked", snapshot);
  // The stale consumer starts on the pre-repair candidate, the commit whose
  // resolution a real deployed lock would carry.
  consumerStale = await stageConsumer("stale", {
    root: repositoryRoot,
    commit: PREREPAIR_COMMIT,
    kind: "commit",
  });

  // 1. Unlocked compatible resolution: the consumer's own fresh lockfile.
  run(rustExec(["cargo", "generate-lockfile"], { manifestPath: join(consumerUnlocked, "Cargo.toml") }), consumerUnlocked);
  const metadata = JSON.parse(
    run(
      rustExec(["cargo", "metadata", "--locked", "--format-version", "1"], {
        manifestPath: join(consumerUnlocked, "Cargo.toml"),
      }),
      consumerUnlocked,
    ),
  );
  assertUnlockedResolution(metadata, snapshot);

  // 2. The unlocked consumer compiles --locked against that resolution, with
  //    private final Cargo outputs on external build scratch.
  run(
    rustExec(
      privateCargoCommand(buildScratch.root, ["cargo", "check", "--locked", "--quiet"]),
      { manifestPath: join(consumerUnlocked, "Cargo.toml") },
    ),
    consumerUnlocked,
  );

  // 3. Stale downstream lock: a consumer whose lock resolved the vulnerable
  //    line against the pre-repair candidate cannot keep it under --locked
  //    once it repoints at the repaired tree, and re-locking lands clean.
  run(rustExec(["cargo", "generate-lockfile"], { manifestPath: join(consumerStale, "Cargo.toml") }), consumerStale);
  const staleLock = await readFile(join(consumerStale, "Cargo.lock"), "utf8");
  if (!staleLock.includes(`name = "age"\nversion = "${VULNERABLE_AGE_VERSION}"`)) {
    throw new Error(`the stale lock did not pin age ${VULNERABLE_AGE_VERSION}; fixture scenario is broken`);
  }
  await writeFile(join(consumerStale, "Cargo.toml"), consumerManifest(snapshot));
  const lockedRefusal = attempt(
    rustExec(["cargo", "check", "--locked", "--quiet"], { manifestPath: join(consumerStale, "Cargo.toml") }),
    consumerStale,
  );
  if (lockedRefusal.ok) {
    throw new Error(`a stale lock pinning age ${VULNERABLE_AGE_VERSION} still built --locked against the repaired requirement`);
  }
  run(rustExec(["cargo", "update", "-p", "longhorn-config-age"], { manifestPath: join(consumerStale, "Cargo.toml") }), consumerStale);
  const relockedMetadata = JSON.parse(
    run(
      rustExec(["cargo", "metadata", "--locked", "--format-version", "1"], {
        manifestPath: join(consumerStale, "Cargo.toml"),
      }),
      consumerStale,
    ),
  );
  assertUnlockedResolution(relockedMetadata, snapshot);
  run(
    rustExec(
      privateCargoCommand(buildScratch.root, ["cargo", "check", "--locked", "--quiet"]),
      { manifestPath: join(consumerStale, "Cargo.toml") },
    ),
    consumerStale,
  );

  const lockSha256 = sha256(await readFile(join(consumerUnlocked, "Cargo.lock")));
  receipt = {
    schema: "longhorn.config-age-security-receipt.v1",
    outcome: "pass",
    sources: {
      repository: repositoryRoot,
      snapshot: snapshot.kind,
      commit: snapshot.commit,
    },
    resolution: {
      repairedAge: REPAIR_AGE_VERSION,
      repairedAgeCorePrefix: REPAIR_AGE_CORE_PREFIX,
      vulnerableChainAbsent: VULNERABLE_CHAIN,
      gitSourceAtExactCommit: true,
      unlockedLockfileSha256: lockSha256,
    },
    staleLockProbe: {
      pinnedAge: VULNERABLE_AGE_VERSION,
      lockedBuildRefused: true,
      relockResolvedClean: true,
    },
    consumerRustVersion: releaseMsrv(),
    stagedTemporaryRemoved: true,
  };
  console.log(JSON.stringify(receipt, null, 2));
} finally {
  if (consumerUnlocked !== undefined) await rm(consumerUnlocked, { recursive: true, force: true });
  if (consumerStale !== undefined) await rm(consumerStale, { recursive: true, force: true });
  if (snapshot.temp !== undefined) await rm(snapshot.temp, { recursive: true, force: true });
  await buildScratch.dispose();
}
