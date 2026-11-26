import { cargoPackages } from "./cargo-metadata.ts";
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
// Final Cargo outputs for the compile, staged consumers, and dirty-tree
// snapshots all live on external build scratch (via `taskScratch` and
// `stagingDirectory`), never under the system temp root.

import { MSRV } from "./msrv.ts";
import { runSync as run, sha256 } from "./proof-support.ts";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import {
  prepareStagedWorkspace,
  rustExec,
  stagedCargoRunner,
  stagingDirectory,
  taskScratch,
} from "./rust-command.ts";

const REPAIR_AGE_VERSION = "0.11.5";
const REPAIR_AGE_CORE_PREFIX = "0.11.";
export const VULNERABLE_AGE_VERSION = "0.12.1";
const VULNERABLE_CHAIN = ["crypto-bigint", "elliptic-curve", "hpke", "ml-kem", "p256"] as const;
const CONSUMED_CRATES = ["longhorn-config-age", "longhorn-config"] as const;
// The pre-repair candidate whose resolution a real deployed consumer lock
// would carry; the stale-lock probe starts there (task longhorn#086).
const PREREPAIR_COMMIT = "1e1f570c88091b7a9196438bc05e140071babc33";

const repositoryRoot = resolveRepository(import.meta.dir);

function resolveRepository(root: string): string {
  const probe = spawnSync("git", ["rev-parse", "--show-toplevel"], { cwd: root, encoding: "utf8" });
  if (probe.status !== 0) {
    throw new Error(`no git repository above ${root}`);
  }
  return probe.stdout.trim();
}

export type AttemptResult = {
  readonly ok: boolean;
  readonly exitCode: number | null;
  readonly output: string;
  readonly error?: string;
};

function attempt(command: ReadonlyArray<string>, cwd: string): AttemptResult {
  const [program, ...args] = command;
  if (!program) {
    throw new Error("expected a command to run");
  }
  const result = spawnSync(program, args, {
    cwd,
    encoding: "utf8",
    env: process.env,
  });
  const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`.trim();
  if (result.error) {
    return {
      ok: false,
      exitCode: result.status,
      output,
      error: result.error.message,
    };
  }
  return {
    ok: result.status === 0,
    exitCode: result.status,
    output,
  };
}

/**
 * Verifies that a `--locked` failure happened specifically because Cargo
 * refused to update a stale lockfile, not from an Oxide, Git or network error.
 */
export function assertLockedRefusal(
  result: AttemptResult,
  vulnerableVersion = VULNERABLE_AGE_VERSION,
): void {
  if (result.ok) {
    throw new Error(
      `a stale lock pinning age ${vulnerableVersion} still built --locked against the repaired requirement`,
    );
  }
  if (result.error !== undefined) {
    throw new Error(`stale-lock check failed to launch: ${result.error}`);
  }
  if (!/lock file\b[\s\S]*--locked was passed to prevent this/i.test(result.output)) {
    throw new Error(
      `stale-lock check failed for an unexpected reason (exit ${String(result.exitCode)}):\n${result.output}`,
    );
  }
}

type Snapshot = { root: string; commit: string; kind: "commit" | "snapshot"; stageRoot?: string };

async function snapshotWorkingTree(): Promise<Snapshot> {
  const stageRoot = await stagingDirectory("longhorn-config-age-security-source");
  try {
    const output = run(
      [
        "bash",
        resolve(import.meta.dir, "git-snapshot.sh"),
        join(stageRoot, "source"),
        "Config age security verification snapshot",
      ],
      repositoryRoot,
    ).trim();
    const lines = output.split("\n");
    if (lines.length !== 3 || (lines[0] !== "commit" && lines[0] !== "snapshot")) {
      throw new Error(`unexpected git-snapshot output:\n${output}`);
    }
    const [kind, root, commit] = lines;
    if (!root || !commit) {
      throw new Error(`incomplete git-snapshot output:\n${output}`);
    }
    return { root, commit, kind, stageRoot };
  } catch (error) {
    await rm(stageRoot, { recursive: true, force: true });
    throw error;
  }
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
rust-version = "${MSRV}"
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

function assertUnlockedResolution(metadata: unknown, snapshot: Snapshot): void {
  const packages = cargoPackages(metadata);
  const byName = new Map(packages.map((pkg) => [pkg.name, pkg]));

  const longhorn = packages.filter(
    (pkg) =>
      pkg.name.startsWith("longhorn-") && pkg.name !== "longhorn-config-age-security-consumer",
  );
  if (longhorn.length < CONSUMED_CRATES.length + 1) {
    throw new Error(
      `expected at least ${CONSUMED_CRATES.length + 1} Longhorn packages (consumers plus transitive core), got ${longhorn.length}`,
    );
  }
  for (const pkg of longhorn) {
    if (
      pkg.source === null ||
      !(pkg.source.startsWith("git+file://") && pkg.source.endsWith(`#${snapshot.commit}`))
    ) {
      throw new Error(
        `${pkg.name} did not resolve from git at the exact snapshot commit: ${pkg.source}`,
      );
    }
  }

  const age = byName.get("age");
  if (age === undefined || age.version !== REPAIR_AGE_VERSION) {
    throw new Error(`age did not resolve to ${REPAIR_AGE_VERSION}: ${age?.version ?? "absent"}`);
  }
  const ageCore = byName.get("age-core");
  if (ageCore === undefined || !ageCore.version.startsWith(REPAIR_AGE_CORE_PREFIX)) {
    throw new Error(
      `age-core did not resolve on the ${REPAIR_AGE_CORE_PREFIX}x line: ${ageCore?.version ?? "absent"}`,
    );
  }
  for (const banned of VULNERABLE_CHAIN) {
    if (byName.has(banned)) {
      throw new Error(`the RUSTSEC-2026-0387 chain still resolves ${banned}`);
    }
  }
}

async function stageConsumer(label: string, snapshot: Snapshot): Promise<string> {
  const directory = await stagingDirectory(`longhorn-config-age-security-${label}`);
  await prepareStagedWorkspace(directory);
  await writeFile(join(directory, "Cargo.toml"), consumerManifest(snapshot));
  const sourceDirectory = join(directory, "src");
  await mkdir(sourceDirectory);
  await writeFile(join(sourceDirectory, "main.rs"), CONSUMER_MAIN);
  return directory;
}

export type SecurityReceipt = {
  readonly schema: "longhorn.config-age-security-receipt.v1";
  readonly outcome: "pass";
  readonly sources: {
    readonly repository: string;
    readonly snapshot: Snapshot["kind"];
    readonly commit: string;
  };
  readonly resolution: {
    readonly repairedAge: string;
    readonly repairedAgeCorePrefix: string;
    readonly vulnerableChainAbsent: ReadonlyArray<string>;
    readonly gitSourceAtExactCommit: boolean;
    readonly unlockedLockfileSha256: string;
  };
  readonly staleLockProbe: {
    readonly pinnedAge: string;
    readonly lockedBuildRefused: boolean;
    readonly relockResolvedClean: boolean;
  };
  readonly consumerRustVersion: string;
  readonly stagedTemporaryRemoved: boolean;
};

export type ReceiptSummary = Omit<SecurityReceipt, "stagedTemporaryRemoved">;

/** Removes each staged workspace directory and reports whether all are absent afterward. */
export async function removeStagedDirectories(
  paths: ReadonlyArray<string | undefined>,
  remove: (path: string) => Promise<void> = (path) => rm(path, { recursive: true, force: true }),
): Promise<boolean> {
  const targets = paths.filter((path): path is string => path !== undefined);
  for (const path of targets) {
    await remove(path);
  }
  return targets.every((path) => !existsSync(path));
}

/**
 * Runs a proof step, removes its staged directories in `finally`, and derives
 * `stagedTemporaryRemoved` from the completed cleanup before emitting the receipt.
 */
export async function runWithStagedCleanup(
  execute: () => Promise<ReceiptSummary>,
  cleanupPaths: () => ReadonlyArray<string | undefined>,
  dispose: () => Promise<void> = async () => {},
): Promise<SecurityReceipt> {
  let summary: ReceiptSummary | undefined;
  let stagedTemporaryRemoved: boolean;
  try {
    summary = await execute();
  } finally {
    stagedTemporaryRemoved = await removeStagedDirectories(cleanupPaths());
    await dispose();
  }
  if (summary === undefined) {
    throw new Error("config-age security proof completed without a receipt");
  }
  return {
    ...summary,
    stagedTemporaryRemoved,
  };
}

async function main(): Promise<void> {
  let snapshot: Snapshot | undefined;
  let buildScratch: Awaited<ReturnType<typeof taskScratch>> | undefined;
  let consumerUnlocked: string | undefined;
  let consumerStale: string | undefined;

  const receipt = await runWithStagedCleanup(
    async () => {
      const currentSnapshot = await snapshotWorkingTree();
      snapshot = currentSnapshot;
      buildScratch = await taskScratch("config-age-security-consumer");

      consumerUnlocked = await stageConsumer("unlocked", currentSnapshot);
      // The stale consumer starts on the pre-repair candidate, the commit whose
      // resolution a real deployed lock would carry.
      consumerStale = await stageConsumer("stale", {
        root: repositoryRoot,
        commit: PREREPAIR_COMMIT,
        kind: "commit",
      });

      const unlockedStaged = await stagedCargoRunner(consumerUnlocked, {
        target: buildScratch.root,
      });
      const staleStaged = await stagedCargoRunner(consumerStale, { target: buildScratch.root });

      // 1. Unlocked compatible resolution: the consumer's own fresh lockfile.
      run(
        rustExec(["cargo", "generate-lockfile"], {
          manifestPath: join(consumerUnlocked, "Cargo.toml"),
        }),
        consumerUnlocked,
      );
      const metadata: unknown = JSON.parse(
        run(
          rustExec(["cargo", "metadata", "--locked", "--format-version", "1"], {
            manifestPath: join(consumerUnlocked, "Cargo.toml"),
          }),
          consumerUnlocked,
        ),
      );
      assertUnlockedResolution(metadata, currentSnapshot);

      // 2. The unlocked consumer compiles --locked against that resolution, with
      //    private final Cargo outputs on external build scratch.
      run(unlockedStaged(["cargo", "check", "--locked", "--quiet"]), consumerUnlocked);

      // 3. Stale downstream lock: a consumer whose lock resolved the vulnerable
      //    line against the pre-repair candidate cannot keep it under --locked
      //    once it repoints at the repaired tree, and re-locking lands clean.
      run(
        rustExec(["cargo", "generate-lockfile"], {
          manifestPath: join(consumerStale, "Cargo.toml"),
        }),
        consumerStale,
      );
      const staleLock = await readFile(join(consumerStale, "Cargo.lock"), "utf8");
      if (!staleLock.includes(`name = "age"\nversion = "${VULNERABLE_AGE_VERSION}"`)) {
        throw new Error(
          `the stale lock did not pin age ${VULNERABLE_AGE_VERSION}; fixture scenario is broken`,
        );
      }
      await writeFile(join(consumerStale, "Cargo.toml"), consumerManifest(currentSnapshot));
      const lockedRefusal = attempt(
        rustExec(["cargo", "check", "--locked", "--quiet"], {
          manifestPath: join(consumerStale, "Cargo.toml"),
        }),
        consumerStale,
      );
      assertLockedRefusal(lockedRefusal, VULNERABLE_AGE_VERSION);

      run(
        rustExec(["cargo", "update", "-p", "longhorn-config-age"], {
          manifestPath: join(consumerStale, "Cargo.toml"),
        }),
        consumerStale,
      );
      const relockedMetadata: unknown = JSON.parse(
        run(
          rustExec(["cargo", "metadata", "--locked", "--format-version", "1"], {
            manifestPath: join(consumerStale, "Cargo.toml"),
          }),
          consumerStale,
        ),
      );
      assertUnlockedResolution(relockedMetadata, currentSnapshot);
      run(staleStaged(["cargo", "check", "--locked", "--quiet"]), consumerStale);

      const lockSha256 = sha256(await readFile(join(consumerUnlocked, "Cargo.lock")));
      return {
        schema: "longhorn.config-age-security-receipt.v1",
        outcome: "pass",
        sources: {
          repository: repositoryRoot,
          snapshot: currentSnapshot.kind,
          commit: currentSnapshot.commit,
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
        consumerRustVersion: MSRV,
      };
    },
    () => [consumerUnlocked, consumerStale, snapshot?.stageRoot],
    async () => {
      if (buildScratch !== undefined) {
        await buildScratch.dispose();
      }
    },
  );

  console.log(JSON.stringify(receipt, null, 2));
}

if (import.meta.main) {
  await main();
}
