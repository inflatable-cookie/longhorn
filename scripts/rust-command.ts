import { isRecord } from "../packages/longhorn/src/core/record.ts";
import { parseJsonRecord, requireStrings } from "./proof-json.ts";
import { errorCode } from "./proof-support.ts";
// Scripts that run Cargo themselves build their argv here instead of naming a
// runner. `scripts/rust-command.sh` is Longhorn's one switch point (`oxide`
// locally, plain `cargo` when `GITHUB_ACTIONS=true` or under
// `EFFIGY_OXIDE_ESCAPE=cargo`), so the CI-aware rule, the exec form and the
// loud missing-Oxide failure have exactly one implementation. This module adds
// no rule of its own; it only names that script and the helpers that keep a
// disposable workspace's build, and its private output, inside the supported
// storage recipe. Run it as `bun scripts/rust-command.ts scratch <label>` to
// print a private scratch directory for a shell route.

import { lstat, mkdir, mkdtemp, realpath, rm, stat } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";

const repositoryRoot = resolve(import.meta.dir, "..");
const switchPoint = join(repositoryRoot, "scripts", "rust-command.sh");
const privateCargo = join(repositoryRoot, "scripts", "private-cargo.sh");

type OxideProfile = "quick" | "debug" | "release";

/**
 * Full argv for the given Cargo arguments, run through the switch point.
 *
 * `toolchain` mirrors `cargo +<toolchain>`: Oxide refuses a `+` argument, so
 * the pin travels as `RUSTUP_TOOLCHAIN`, which is also what `cargo` reads when
 * the same argv runs on a runner.
 */
export function rustCommand(args: ReadonlyArray<string>, toolchain?: string): Array<string> {
  const command = ["bash", switchPoint, ...args];
  return toolchain === undefined ? command : ["env", `RUSTUP_TOOLCHAIN=${toolchain}`, ...command];
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
export function rustExec(command: ReadonlyArray<string>, options: RustExecOptions): Array<string> {
  const router = ["bash", switchPoint, "exec", "--manifest-path", options.manifestPath];
  if (options.profile !== undefined) {
    router.push("--profile", options.profile);
  }
  router.push("--", ...command);
  return options.toolchain === undefined
    ? router
    : ["env", `RUSTUP_TOOLCHAIN=${options.toolchain}`, ...router];
}

/**
 * Full argv that runs one disposable workspace's Cargo with private final
 * outputs under `target`.
 */
export function privateCargoCommand(target: string, args: ReadonlyArray<string>): Array<string> {
  return ["bash", privateCargo, "--target", target, "--", ...args];
}

/**
 * Library archive paths Cargo reports for `packageName`'s lib target, read from
 * `cargo ... --message-format=json` output. Cargo chooses where its artifacts
 * land, and that layout is not stable across builds (an rlib is uplifted to
 * `<target>/<profile>/`, not `deps/`), so a scan reads the files this build
 * reported rather than assuming a directory.
 */
export function libraryArchives(messages: string, packageName: string): Array<string> {
  const libraryName = packageName.replaceAll("-", "_");
  const archives: Array<string> = [];
  for (const line of messages.split("\n")) {
    if (!line.startsWith("{")) {
      continue;
    }
    const message = parseJsonRecord(line);
    if (message.reason !== "compiler-artifact") {
      continue;
    }
    const target = message.target;
    if (!isRecord(target) || target.name !== libraryName) {
      continue;
    }
    if (!requireStrings(target.kind ?? [], "Cargo target kind").includes("lib")) {
      continue;
    }
    archives.push(
      ...requireStrings(message.filenames ?? [], "Cargo artifact filenames").filter((file) =>
        file.endsWith(".rlib"),
      ),
    );
  }
  return archives;
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

const scratchVariable = "OXIDE_TASK_SCRATCH_ROOT";

/**
 * Allocates one disposable private Cargo output root on external scratch.
 *
 * Local build scratch lives under `OXIDE_TASK_SCRATCH_ROOT`, or under the
 * scratch root Oxide reports for its registered removable build pool, in its
 * `longhorn` child. It is never on the boot volume, the account home or a drive
 * root. The root must already exist: a missing build volume fails closed here
 * instead of becoming an ordinary directory on the internal drive. Hosted CI
 * keeps its runner-owned temporary storage.
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
  if (process.env.GITHUB_ACTIONS === "true") {
    return tmpdir();
  }
  const configured = process.env[scratchVariable];
  const explicit = configured !== undefined && configured !== "";
  const owner = explicit ? resolve(configured) : await reportedScratchRoot();
  await assertExternalScratch(owner);
  const base = explicit ? owner : join(owner, "longhorn");
  try {
    await mkdir(base);
  } catch (error) {
    if (errorCode(error) !== "EEXIST") {
      throw new Error(
        `cannot create build scratch ${base}: ${String(error)}. Its parent must already be an external build scratch root.`,
        { cause: error },
      );
    }
  }
  return base;
}

const refusal = (why: string) =>
  new Error(
    `no build scratch root: ${why}. Set ${scratchVariable} to an existing directory on a build volume; nothing is created on the internal drive.`,
  );

/**
 * The scratch root Oxide reports for its registered removable build pool.
 * Oxide owns that fact, so no volume name is written here. Every gap refuses
 * and names `OXIDE_TASK_SCRATCH_ROOT`, the operator's way to set it.
 */
async function reportedScratchRoot(): Promise<string> {
  let proc: Bun.Subprocess<"ignore", "pipe", "pipe">;
  try {
    // An explicit environment, so `oxide` is looked up on the current PATH.
    proc = Bun.spawn(["oxide", "pool", "describe", "--json"], {
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
      env: { ...process.env },
    });
  } catch (error) {
    throw refusal(`cannot run oxide pool describe --json (${String(error)})`);
  }
  const [code, output, errors] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  if (code !== 0) {
    throw refusal(`oxide pool describe --json exited ${code}: ${errors.trim()}`);
  }

  let report: Record<string, unknown>;
  try {
    report = parseJsonRecord(output);
  } catch {
    throw refusal("oxide pool describe --json printed no JSON");
  }
  const removable = isRecord(report.removable) ? report.removable : undefined;
  if (removable === undefined || removable.available !== true) {
    const reason =
      typeof removable?.unavailable_reason === "string" ? ` (${removable.unavailable_reason})` : "";
    throw refusal(`oxide reports no available removable build pool${reason}`);
  }
  const root = removable.suggested_scratch_root;
  if (typeof root !== "string" || root === "") {
    throw refusal("oxide reports no suggested scratch root for the removable pool");
  }
  return resolve(root);
}

export type ScratchLocation = { readonly path: string; readonly device: number };

/** Facts about the boot volume: its device numbers and the account home. */
export type BootVolume = { readonly devices: ReadonlyArray<number>; readonly home: string };

/**
 * The reason a resolved scratch location is refused, or `null` when it is
 * external scratch. The internal drive is judged by volume (`device`), never by
 * `TMPDIR`, which Nucleus points at external scratch. Pure, so the rule is
 * tested without depending on this host's drives.
 */
export function scratchRefusal(location: ScratchLocation, boot: BootVolume): string | null {
  const { path, device } = location;
  // `dirname("/")` is `/`, so the drive root is tested before its children.
  if (path === sep) {
    return "a drive root";
  }
  if (dirname(path) === sep) {
    return "a boot-volume top-level directory";
  }
  if (/^\/Volumes\/[^/]+$/.test(path)) {
    return "a volume mount root";
  }
  if (path === boot.home || path.startsWith(`${boot.home}${sep}`)) {
    return "the account home";
  }
  if (boot.devices.includes(device)) {
    return "on the boot volume, the internal drive";
  }
  return null;
}

async function bootVolume(): Promise<BootVolume> {
  const home = await realpath(homedir()).catch(() => homedir());
  const devices = await Promise.all([sep, home].map(async (path) => (await stat(path)).dev));
  return { devices, home };
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
      `build scratch ${owner} does not exist. No local build output is created on the internal drive, the account home or a drive root; set ${scratchVariable} to an existing directory on a build volume.`,
    );
  }
  const info = await stat(resolved);
  if (!info.isDirectory()) {
    throw new Error(`build scratch ${resolved} is not a directory`);
  }

  const refusal = scratchRefusal({ path: resolved, device: info.dev }, await bootVolume());
  if (refusal !== null) {
    throw new Error(
      `build scratch ${resolved} is ${refusal}. Local build outputs stay on external build scratch, never there.`,
    );
  }
}

if (import.meta.main) {
  const [command, label, ...extra] = process.argv.slice(2);
  if (
    command !== "scratch" ||
    label === undefined ||
    extra.length > 0 ||
    !/^[A-Za-z0-9._-]+$/.test(label)
  ) {
    console.error("usage: external-scratch.sh <label>");
    process.exit(2);
  }
  try {
    console.log((await taskScratch(label)).root);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
