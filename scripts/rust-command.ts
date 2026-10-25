// Scripts that run Cargo themselves build their argv here instead of naming a
// runner. `scripts/rust-command.sh` is Longhorn's one switch point (`oxide`
// locally, plain `cargo` when `GITHUB_ACTIONS=true` or under
// `EFFIGY_OXIDE_ESCAPE=cargo`), so the CI-aware rule, the exec form and the
// loud missing-Oxide failure have exactly one implementation. This module adds
// no rule of its own; it only names that script and the two helpers that keep a
// disposable workspace's build inside the supported storage recipe.

import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dir, "..");
const switchPoint = join(repositoryRoot, "scripts", "rust-command.sh");
const privateCargo = join(repositoryRoot, "scripts", "private-cargo.sh");

export type OxideProfile = "quick" | "debug" | "release";

/**
 * Full argv for the given Cargo arguments, run through the switch point.
 *
 * `toolchain` mirrors `cargo +<toolchain>`: Oxide refuses a `+` argument, so
 * the pin travels as `RUSTUP_TOOLCHAIN`, which is also what `cargo` reads when
 * the same argv runs on a runner.
 */
export function rustCommand(
  args: readonly string[],
  toolchain?: string,
): string[] {
  const command = ["bash", switchPoint, ...args];
  return toolchain === undefined
    ? command
    : ["env", `RUSTUP_TOOLCHAIN=${toolchain}`, ...command];
}

export type RustExecOptions = {
  /** The one Cargo workspace this lease covers. */
  readonly manifestPath: string;
  /** Oxide's recipe and slot only; the tool still picks its own dev or release build. */
  readonly profile?: OxideProfile;
  /** Pins `RUSTUP_TOOLCHAIN` before the lease, as the former `cargo +<pin>` did. */
  readonly toolchain?: string;
};

/**
 * Full argv for one tool that calls Cargo itself, run through the switch
 * point's `exec` form.
 *
 * One lease covers one workspace and one toolchain: a proof that stages several
 * consumers calls this per staged workspace with that workspace's
 * `--manifest-path`, and the tool inside calls plain `cargo`, never Oxide.
 */
export function rustExec(
  command: readonly string[],
  options: RustExecOptions,
): string[] {
  const router = [
    "bash",
    switchPoint,
    "exec",
    "--manifest-path",
    options.manifestPath,
  ];
  if (options.profile !== undefined) router.push("--profile", options.profile);
  router.push("--", ...command);
  return options.toolchain === undefined
    ? router
    : ["env", `RUSTUP_TOOLCHAIN=${options.toolchain}`, ...router];
}

/**
 * Full argv that runs one disposable workspace's Cargo with private final
 * outputs under `target`.
 *
 * `buildDir` pins `CARGO_BUILD_BUILD_DIR` as well, for the two release-absence
 * scans that read Cargo's own `target/<profile>/deps` layout.
 */
export function privateCargoCommand(
  target: string,
  args: readonly string[],
  options: { readonly buildDir?: boolean } = {},
): string[] {
  const command = ["bash", privateCargo, "--target", target];
  if (options.buildDir === true) command.push("--build-dir");
  command.push("--", ...args);
  return command;
}

export type TaskScratch = {
  /** Root for this proof's private final Cargo outputs. */
  readonly root: string;
  dispose(): Promise<void>;
};

/**
 * Allocates one disposable private Cargo output root on external scratch.
 *
 * Local build scratch lives on `OXIDE_TASK_SCRATCH_ROOT` (or
 * `/Volumes/Builds/scratch/longhorn`), never on the internal drive, the account
 * home or a drive root. Hosted CI keeps its runner-owned temporary storage.
 */
export async function taskScratch(label: string): Promise<TaskScratch> {
  const base = scratchBase();
  try {
    await mkdir(base, { recursive: true });
  } catch (error) {
    throw new Error(
      `cannot reach build scratch ${base}: ${String(error)}. Set OXIDE_TASK_SCRATCH_ROOT to a writable directory on a build volume.`,
    );
  }
  const root = await mkdtemp(join(base, `${label}-`));
  return {
    root,
    dispose: async () => {
      await rm(root, { recursive: true, force: true });
    },
  };
}

function scratchBase(): string {
  if (process.env.GITHUB_ACTIONS === "true") return tmpdir();
  const configured = process.env.OXIDE_TASK_SCRATCH_ROOT;
  return configured !== undefined && configured !== ""
    ? resolve(configured)
    : "/Volumes/Builds/scratch/longhorn";
}
