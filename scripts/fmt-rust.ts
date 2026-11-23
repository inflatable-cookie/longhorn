import { readCargoInventory } from "./cargo-metadata.ts";
// Format-check all Rust sources in the repository:
// 1. Root workspace members via `cargo metadata --no-deps` + `cargo fmt --check -p ...`
//    (never `--all`, which would walk relative path dependencies into sibling repos).
// 2. All standalone `prototypes/*/` workspaces via `cargo fmt --check`.
// 3. Staged example template crates (`examples/<group>/**`), whose Cargo.toml
//    manifests inherit `*.workspace = true` from a temporary workspace created at
//    proof runtime, so `cargo fmt` cannot resolve them in-place; `rustfmt` checks
//    their `.rs` files directly under edition 2024.
//
// All Cargo invocations run through `scripts/rust-command.sh` (Oxide passthrough
// locally, plain Cargo on GitHub Actions or under `EFFIGY_OXIDE_ESCAPE=cargo`).

import { readdir } from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import { rustCommand } from "./rust-command.ts";

const repoRoot = resolve(import.meta.dir, "..");
const stagedExampleRoots = [
  "examples/desktop-support-conformance",
  "examples/history-system-proof",
  "examples/operation-notification-proof",
  "examples/native-content-system-proof",
  "examples/history-tree-artifact-proof",
  "examples/greenfield-compositions",
] as const;

const writeMode = process.argv.includes("--write");

type CommandResult = {
  exitCode: number;
  stdout: string;
  stderr: string;
};

async function runChecked(command: ReadonlyArray<string>, cwd: string): Promise<CommandResult> {
  const proc = Bun.spawn([...command], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  if (exitCode !== 0) {
    const output = [stdout.trim(), stderr.trim()].filter(Boolean).join("\n");
    throw new Error(
      `${command.join(" ")} failed in ${relative(repoRoot, cwd) || "."} (exit ${exitCode})\n${output}`,
    );
  }
  return { exitCode, stdout, stderr };
}

async function listRsFiles(directory: string): Promise<Array<string>> {
  const files: Array<string> = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === "target" || entry.name === "gen") {
      continue;
    }
    const full = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await listRsFiles(full)));
    } else if (entry.isFile() && entry.name.endsWith(".rs")) {
      files.push(full);
    }
  }
  return files;
}

const fmtFlags = writeMode ? [] : ["--check"];

// 1. Root workspace members
const metadataResult = await runChecked(
  rustCommand(["metadata", "--no-deps", "--format-version", "1"]),
  repoRoot,
);
const metadata = readCargoInventory(metadataResult.stdout);
const packageFlags = metadata.packages.map((pkg) => `-p${pkg.name}`);
if (packageFlags.length === 0) {
  throw new Error("cargo metadata --no-deps returned zero workspace packages");
}
await runChecked(rustCommand(["fmt", ...fmtFlags, ...packageFlags]), repoRoot);

// 2. Prototype workspaces
const prototypesDir = join(repoRoot, "prototypes");
const prototypeDirs = (await readdir(prototypesDir, { withFileTypes: true }))
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort();
if (prototypeDirs.length === 0) {
  throw new Error("no prototypes/* workspaces found");
}
for (const name of prototypeDirs) {
  await runChecked(rustCommand(["fmt", ...fmtFlags]), join(prototypesDir, name));
}

// 3. Staged example template sources
const exampleFiles: Array<string> = [];
for (const root of stagedExampleRoots) {
  exampleFiles.push(...(await listRsFiles(join(repoRoot, root))));
}
exampleFiles.sort();
if (exampleFiles.length === 0) {
  throw new Error("no Rust files found in staged example roots");
}
await runChecked(["rustfmt", "--edition", "2024", ...fmtFlags, ...exampleFiles], repoRoot);

console.log(
  JSON.stringify({
    schema: "longhorn.fmt-rust.v1",
    outcome: "pass",
    mode: writeMode ? "write" : "check",
    workspacePackages: packageFlags.length,
    prototypeWorkspaces: prototypeDirs.length,
    stagedExampleFiles: exampleFiles.length,
  }),
);
