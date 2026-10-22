// Desktop support second-consumer conformance (contract 003 step 6).
//
// Contracts 024 to 028 each need a second consumer's conformance before their
// extraction is done. This proof is that consumer, and it is deliberately not
// another Longhorn test: the six desktop support crates are copied out of the
// workspace, a neutral consumer crate beside them is built from their packaged
// sources only, and every assertion runs through their public APIs.
//
// What makes it a second consumer rather than a quieter copy of the crates'
// own suites:
//
// - The consumer lives under `examples/desktop-support-conformance/`, is not a
//   root workspace member, and is built in a disposable private workspace
//   whose manifest names only the six crates.
// - It imports no workspace path, reads no fixture from this repository, and
//   carries no product value: every secret, capacity, bound, rule set, section
//   and release query is its own.
// - It runs the live paths Acowtancy's adoption only compiled: a real tool
//   install against the crate's loopback `fixture-server` (with a real
//   `CommandRunner` version probe), and a real `/bin/sh` PTY session.
//
// Unix hosts only: the PTY unit starts real child processes and the install
// unit needs POSIX file modes. The fixture server stands in for GitHub, so no
// request leaves the host and no release, token or code signature is involved.
//
// A cold run compiles the six crates and their dependency graph in the private
// workspace, which is minutes; with a warm compiler cache it is seconds. Either
// way it is a standalone selector, not a member of `qa`. Keep the stage for
// inspection with KEEP_DESKTOP_SUPPORT_CONFORMANCE=1.

import { spawnSync } from "node:child_process";
import { cp, lstat, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const consumerName = "longhorn-desktop-support-conformance";
const exampleRoot = join(repoRoot, "examples/desktop-support-conformance");
const consumerRoot = join(exampleRoot, "consumer");

/** The crates the proof stages. Contract 027's shared path rule travels with it. */
const crates = [
  "longhorn-archive-path",
  "longhorn-diagnostics",
  "longhorn-pty",
  "longhorn-redact",
  "longhorn-support-log",
  "longhorn-tool-provision",
] as const;

/** The units the consumer asserts, in the order it runs them. */
const units = ["024", "025", "026", "027", "028"] as const;

/** The crate each unit exercises. */
const unitCrates: Record<(typeof units)[number], readonly string[]> = {
  "024": ["longhorn-redact"],
  "025": ["longhorn-support-log"],
  "026": ["longhorn-diagnostics"],
  "027": ["longhorn-tool-provision"],
  "028": ["longhorn-pty"],
};

const platform = process.platform;
if (platform !== "darwin" && platform !== "linux") {
  console.error(
    `proof:desktop-support-conformance runs on Unix hosts only; this host is ${platform}.`,
  );
  process.exit(1);
}

const stage = await mkdtemp(join(tmpdir(), "longhorn-desktop-support-conformance-"));

try {
  await verifyExampleSources();
  const inventories = crates.map((name) => verifyPackageInventory(name));

  for (const name of crates) {
    await cp(join(repoRoot, "crates", name), join(stage, "crates", name), { recursive: true });
  }
  await cp(consumerRoot, join(stage, "consumer"), { recursive: true });
  await writeFile(join(stage, "Cargo.toml"), await privateWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(stage, "Cargo.lock"));

  const startedAt = performance.now();
  const output = run(["cargo", "run", "-p", consumerName, "--offline", "--quiet"], stage);
  const seconds = Number(((performance.now() - startedAt) / 1000).toFixed(1));
  const lines = output.trim().split("\n");
  if (!lines.includes("outcome: pass")) {
    throw new Error(`the consumer did not report a pass:\n${output}`);
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.desktop-support-conformance.v1",
        outcome: "pass",
        platform,
        crates: [...crates],
        packages: inventories,
        consumer: {
          package: consumerName,
          units: [...units],
          runSeconds: seconds,
          output: lines,
        },
        proofLimits: [
          "Unix hosts only: the PTY unit starts real /bin/sh children and the install unit needs POSIX file modes",
          "The crate's loopback fixture server stands in for GitHub: no public network, no published release, no API token and no code signature",
          "Crates are staged from their packaged sources into a disposable workspace; nothing is installed from a registry",
          "Runs one contract row set per unit; it does not repeat the crates' own suites",
        ],
      },
      null,
      2,
    ),
  );
} catch (error) {
  console.error(String(error instanceof Error ? error.stack ?? error.message : error));
  process.exitCode = 1;
} finally {
  if (process.env.KEEP_DESKTOP_SUPPORT_CONFORMANCE === "1") {
    console.error(`retained conformance stage: ${stage}`);
  } else {
    await rm(stage, { recursive: true, force: true });
  }
}

/**
 * The consumer must stay a template, not a workspace member, and must carry no
 * product vocabulary. Both are what make the proof a second consumer.
 */
async function verifyExampleSources(): Promise<void> {
  const root = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  if (root.includes("desktop-support-conformance")) {
    throw new Error("the conformance consumer must not be a root workspace member");
  }

  const manifest = await readFile(join(consumerRoot, "Cargo.toml"), "utf8");
  const declared = [...manifest.matchAll(/^(longhorn-[\w-]+)(?:\.workspace)?\s*=/gm)].map(
    (match) => match[1]!,
  );
  for (const name of declared) {
    if (!(crates as readonly string[]).includes(name)) {
      throw new Error(`the consumer depends on ${name}, which is not a desktop support crate`);
    }
  }
  // The five unit crates are direct dependencies. `longhorn-archive-path` is
  // staged but not imported here: it is contract 027's shared path rule, which
  // travels with `longhorn-tool-provision`.
  for (const name of units.flatMap((unit) => unitCrates[unit])) {
    if (!declared.includes(name)) throw new Error(`the consumer does not depend on ${name}`);
  }

  // Longhorn's own names are fine; another repository's product vocabulary is
  // not, because the consumer must not copy a donor's product policy. The README
  // may name the consumer history; the crate the private workspace builds may
  // not.
  const productVocabulary = /acowtancy|bovine|praxis|jetstream|loophole|nucleus|soundcheck|split-shell/i;
  const offenders: string[] = [];
  for (const relative of await readdir(consumerRoot, { recursive: true })) {
    if (!(await lstat(join(consumerRoot, relative))).isFile()) continue;
    for (const [index, line] of (await readFile(join(consumerRoot, relative), "utf8")).split("\n").entries()) {
      if (productVocabulary.test(line)) {
        offenders.push(`${relative}:${index + 1}: ${line.trim()}`);
      }
    }
  }
  if (offenders.length > 0) {
    throw new Error(`the conformance consumer carries product vocabulary:\n${offenders.join("\n")}`);
  }
}

/**
 * `cargo package --list` is the packaged inventory: what a consumer taking the
 * crate by tag actually receives. The proof copies crate sources, so this is
 * the check that those sources are the ones a package ships.
 */
function verifyPackageInventory(name: string): { name: string; files: number } {
  const listing = run(["cargo", "package", "-p", name, "--list", "--allow-dirty"], repoRoot);
  const files = listing.trim().split("\n");
  for (const required of ["Cargo.toml"]) {
    if (!files.includes(required)) throw new Error(`${name}'s package inventory omits ${required}`);
  }
  if (!files.some((file) => file.startsWith("src/"))) {
    throw new Error(`${name}'s package inventory omits its sources`);
  }
  return { name, files: files.length };
}

/**
 * The private workspace: only the six crates and the consumer, with the
 * repository's `[workspace.package]`, dependency versions and lints. Every
 * `longhorn-*` dependency line outside the selected set is dropped, so nothing
 * the consumer builds can reach a crate this proof does not stage.
 */
async function privateWorkspaceManifest(): Promise<string> {
  const root = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  const tail = root.slice(root.indexOf("[workspace.package]"));
  const version = /\nversion = "([^"]+)"/.exec(tail)?.[1];
  if (!version) throw new Error("the workspace package version is missing");
  const selected = new Set<string>(crates);
  const filtered = tail
    .split("\n")
    .filter((line) => {
      const match = line.match(/^(longhorn-[\w-]+)\s*=/);
      return match === null || selected.has(match[1]!);
    })
    .join("\n");
  // A staged crate with no `[workspace.dependencies]` line in the repository --
  // `longhorn-diagnostics` and `longhorn-pty` have none, because no root member
  // inherits them -- is declared here so the consumer can inherit every staged
  // crate instead of reaching for a sibling path.
  const missing = crates.filter((name) => !new RegExp(`^${name}\\s*=`, "m").test(filtered));
  const additions = missing
    .map((name) => `${name} = { path = "crates/${name}", version = "${version}" }`)
    .join("\n");
  const dependencies = additions.length === 0
    ? filtered
    : filtered.replace("[workspace.dependencies]\n", `[workspace.dependencies]\n${additions}\n`);
  const members = [...crates.map((name) => `  "crates/${name}",`), '  "consumer",'].join("\n");
  return `[workspace]\nmembers = [\n${members}\n]\nresolver = "2"\n\n${dependencies}`;
}

function run(command: readonly string[], cwd: string): string {
  // Plain Cargo, not `scripts/rust-command.sh`: the consumer and its
  // dependencies build in a disposable private workspace that Oxide does not
  // manage, exactly as the greenfield artifact proof does. The package
  // inventories run here because no selector compiles them.
  const result = spawnSync(command[0]!, command.slice(1), {
    cwd,
    encoding: "utf8",
    env: { ...process.env },
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd} (exit ${result.status})\n${result.stdout}\n${result.stderr}`,
    );
  }
  return result.stdout;
}
