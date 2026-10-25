// Scripts that run Cargo themselves build their argv here instead of naming a
// runner. `scripts/rust-command.sh` is Longhorn's one switch point (`oxide`
// locally, plain `cargo` when `GITHUB_ACTIONS=true` or under
// `EFFIGY_OXIDE_ESCAPE=cargo`), so the CI-aware rule, the exec form and the
// loud missing-Oxide failure have exactly one implementation. This module adds
// no rule of its own; it only names that script and the two helpers that keep a
// disposable workspace's build inside the supported storage recipe.

import { lstat, mkdir, mkdtemp, realpath, rm, stat } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";

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

/**
 * Prepares one owned disposable Cargo workspace for an adoption-free lease.
 *
 * Oxide's automatic first conversion turns `<workspace>/target` into an owned
 * output link, which refuses on a staged workspace with no Git work tree to
 * confirm. An existing REAL `target/` directory opts that workspace out of
 * automatic conversion: Oxide reports the outputs as not converted and creates
 * no output root. Never call this for a real checkout, and never replace an
 * existing link: an owned-output link is not this function's to overwrite.
 */
export async function prepareStagedWorkspace(workspace: string): Promise<string> {
  const target = join(workspace, "target");
  const existing = await lstat(target).catch(() => null);
  if (existing !== null) {
    if (!existing.isDirectory()) {
      throw new Error(
        `${target} exists and is not a real directory; refusing to replace it. Staged-workspace preparation never overwrites an existing target or an Oxide owned-output link.`,
      );
    }
    return target;
  }
  await mkdir(target);
  return target;
}

export type TaskScratch = {
  /** Root for this proof's private final Cargo outputs. */
  readonly root: string;
  dispose(): Promise<void>;
};

/**
 * The build volume `scratch` directory. Its own existence is the operator's
 * side of the contract; `longhorn` below it is the repository's, and it is the
 * only directory this module ever creates under it.
 */
const defaultVolumeScratch = "/Volumes/Builds/scratch";

/**
 * Allocates one disposable private Cargo output root on external scratch.
 *
 * Local build scratch lives under `OXIDE_TASK_SCRATCH_ROOT`, or under the build
 * volume's `scratch/longhorn`, never on the internal temp directory, the
 * account home or a drive root. The configured root must already exist: a
 * missing build volume fails closed here instead of becoming an ordinary
 * directory on the internal drive. Hosted CI keeps its runner-owned temporary
 * storage.
 */
export async function taskScratch(label: string): Promise<TaskScratch> {
  const base = await scratchBase();
  const root = await mkdtemp(join(base, `${label}-`));
  return {
    root,
    dispose: async () => {
      await rm(root, { recursive: true, force: true });
    },
  };
}

async function scratchBase(): Promise<string> {
  if (process.env.GITHUB_ACTIONS === "true") return tmpdir();
  const configured = process.env.OXIDE_TASK_SCRATCH_ROOT;
  const owner = configured !== undefined && configured !== "" ? resolve(configured) : defaultVolumeScratch;
  await assertExternalScratch(owner);
  const base = configured !== undefined && configured !== ""
    ? owner
    : join(owner, "longhorn");
  try {
    await mkdir(base);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") {
      throw new Error(
        `cannot create build scratch ${base}: ${String(error)}. Its parent must already be an external build scratch root.`,
      );
    }
  }
  return base;
}

/**
 * Fails closed on a scratch root that would put local build outputs somewhere
 * the recipe forbids. Checked against the resolved path, so an alias cannot
 * slip past it.
 */
async function assertExternalScratch(owner: string): Promise<void> {
  const resolved = await realpath(owner).catch(() => null);
  if (resolved === null) {
    throw new Error(
      `build scratch ${owner} does not exist. No local build output is created on the internal drive, the account home or a drive root; set OXIDE_TASK_SCRATCH_ROOT to an existing directory on a build volume.`,
    );
  }
  if (!(await stat(resolved)).isDirectory()) {
    throw new Error(`build scratch ${resolved} is not a directory`);
  }
  const forbidden = await forbiddenScratchRoots();
  for (const { path, label } of forbidden) {
    if (resolved === path || resolved.startsWith(`${path}${sep}`)) {
      throw new Error(
        `build scratch ${resolved} is ${label}. Local build outputs stay on external build scratch, never there.`,
      );
    }
  }
}

async function forbiddenScratchRoots(): Promise<Array<{ path: string; label: string }>> {
  const [internalTemp, home] = await Promise.all([
    realpath(tmpdir()).catch(() => tmpdir()),
    realpath(homedir()).catch(() => homedir()),
  ]);
  return [
    { path: "/", label: "a drive root" },
    { path: dirname("/x"), label: "a boot-volume top-level directory" },
    { path: internalTemp, label: "the internal temporary directory" },
    { path: home, label: "the account home" },
  ].filter((entry) => entry.path !== "/x");
}
