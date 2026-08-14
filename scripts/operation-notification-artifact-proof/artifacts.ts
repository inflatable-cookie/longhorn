import { poodleRelease } from "../poodle-release.ts";
import { workspaceDependencies } from "../workspace-dependencies.ts";
import { basename, join, resolve } from "node:path";
import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { MSRV, MSRV_TOOLCHAIN } from "../msrv.ts";

import { assertExactSet, digest, parseTrace, readSourceTree, run } from "./shared.ts";
import type { ArtifactIdentity, Shape } from "./types.ts";

// Poodle installs from the registry; poodleRelease() checks each published
// package's sha512 against bun.lock and against the installed copy.
export const POODLE_RELEASE = poodleRelease();

const typescriptPackages = [["@inflatable-cookie/longhorn", "longhorn"], ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"], ["@inflatable-cookie/longhorn-tauri", "longhorn-tauri"]] as const;

const rustCrates = [
  "longhorn-core",
  "longhorn-bridge",
  "longhorn-operation",
  "longhorn-notifications",
  "longhorn-tauri-operation",
  "longhorn-tauri-notifications",
] as const;

export async function packTypescriptArtifacts(repoRoot: string, artifactRoot: string) {
  const paths = new Map<string, string>();
  const identities: ArtifactIdentity[] = [];
  for (const [name, directory] of typescriptPackages) {
    await run(["bun", "pm", "pack", "--destination", artifactRoot, "--ignore-scripts", "--quiet"], join(repoRoot, "packages", directory));
    const path = join(artifactRoot, `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`);
    await inspectTypescriptArtifact(name, path, artifactRoot);
    paths.set(name, path);
    identities.push({ name, filename: basename(path), sha256: await digest(path) });
  }
  return { paths, identities };
}

async function inspectTypescriptArtifact(name: string, path: string, artifactRoot: string): Promise<void> {
  const listing = await run(["tar", "-tzf", path], artifactRoot);
  if (listing.includes("node_modules/") || listing.includes("/tests/") || listing.includes("workspace:")) throw new Error(`${name} artifact contains proof/workspace material`);
  const manifest = JSON.parse(await run(["tar", "-xOzf", path, "package/package.json"], artifactRoot)) as {
    name: string; version: string; dependencies?: Record<string, string>; exports?: Record<string, unknown>;
    peerDependenciesMeta?: Record<string, { optional?: boolean }>;
  };
  if (manifest.name !== name || manifest.version !== "0.1.0") throw new Error(`${name} packed identity mismatch`);
  const dependencies: Record<string, readonly string[]> = {
    "@inflatable-cookie/longhorn": [],
    "@inflatable-cookie/longhorn-poodle-svelte": [],
    "@inflatable-cookie/longhorn-tauri": [],
  };
  assertExactSet(`${name} dependencies`, Object.keys(manifest.dependencies ?? {}), dependencies[name] ?? []);
  if (name === "@inflatable-cookie/longhorn-operation" || name === "@inflatable-cookie/longhorn-notifications") {
    for (const peer of ["svelte", "@inflatable-cookie/poodle-svelte"]) if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) throw new Error(`${name} ${peer} peer is not optional`);
  }
  const extractRoot = join(artifactRoot, `inspect-${name.replace("@", "").replace("/", "-")}`);
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], artifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  for (const marker of ["PulseMutation", "PluginRecord", "ExportReport", "executeProductPayload"]) if (source.includes(marker)) throw new Error(`${name} contains donor payload marker ${marker}`);
}

export async function packAndRunRustArtifacts(repoRoot: string, proofRoot: string, temporaryRoot: string) {
  const artifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const identities: ArtifactIdentity[] = [];
  for (const name of rustCrates) {
    const inventory = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "package", "-p", name, "--list", "--allow-dirty"], repoRoot);
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) throw new Error(`${name} source inventory is incomplete`);
    const archive = join(artifactRoot, `${name}-0.1.0.private.tar.gz`);
    await run(["tar", "-czf", archive, "-C", repoRoot, `crates/${name}`], repoRoot);
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({ name, filename: basename(archive), sha256: await digest(archive) });
  }
  const sharedSource = (await Promise.all(rustCrates.map((name) => readSourceTree(join(workspace, "crates", name, "src"))))).join("\n");
  for (const marker of ["PulseMutation", "PluginRecord", "ExportReport", "executeProductPayload"]) if (sharedSource.includes(marker)) throw new Error(`Rust artifacts contain donor marker ${marker}`);

  const consumersRoot = join(workspace, "consumers");
  await mkdir(consumersRoot);
  const shapes: Shape[] = ["minimal-operation", "soundcheck", "loophole", "notification-only"];
  for (const shape of shapes) await cp(join(proofRoot, "rust", shape), join(consumersRoot, shape), { recursive: true });
  await writeFile(join(workspace, "Cargo.toml"), rustWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));

  const nativeTraces = {} as Record<Shape, Record<string, unknown>>;
  const graphs = {} as Record<Shape, readonly string[]>;
  const packages: Record<Shape, string> = {
    "minimal-operation": "longhorn-minimal-operation-artifact-proof",
    soundcheck: "longhorn-soundcheck-operation-artifact-proof",
    loophole: "longhorn-loophole-operation-notification-artifact-proof",
    "notification-only": "longhorn-notification-only-artifact-proof",
  };
  const expected: Record<Shape, readonly string[]> = {
    "minimal-operation": ["longhorn-core", "longhorn-operation"],
    soundcheck: ["longhorn-core", "longhorn-operation", "longhorn-tauri-operation"],
    loophole: ["longhorn-bridge", "longhorn-core", "longhorn-notifications", "longhorn-operation", "longhorn-tauri-notifications", "longhorn-tauri-operation"],
    "notification-only": ["longhorn-core", "longhorn-notifications", "longhorn-tauri-notifications"],
  };
  for (const shape of shapes) {
    nativeTraces[shape] = parseTrace(await run(["cargo", `+${MSRV_TOOLCHAIN}`, "run", "-p", packages[shape], "--offline", "--quiet"], workspace));
    const tree = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "tree", "-p", packages[shape], "--offline", "--prefix", "none"], workspace);
    graphs[shape] = longhornPackages(tree);
    assertExactSet(`${shape} Rust graph`, graphs[shape], expected[shape]);
  }
  return { identities, graphs, nativeTraces, packaging: { privateSourceInventories: true, registryNormalizedCrates: false, cargoPackageListPassed: true } };
}

function longhornPackages(tree: string): readonly string[] {
  return [...new Set(tree.split("\n").map((line) => line.trim().split(/\s+/)[0] ?? "").filter((name) => name.startsWith("longhorn-") && !name.endsWith("-artifact-proof")))].sort();
}

function rustWorkspaceManifest(): string {
  return `[workspace]
members = ["consumers/minimal-operation", "consumers/soundcheck", "consumers/loophole", "consumers/notification-only", "crates/longhorn-core", "crates/longhorn-bridge", "crates/longhorn-operation", "crates/longhorn-notifications", "crates/longhorn-tauri-operation", "crates/longhorn-tauri-notifications"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "${MSRV}"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-bridge = { path = "crates/longhorn-bridge", version = "0.1.0" }
longhorn-operation = { path = "crates/longhorn-operation", version = "0.1.0" }
longhorn-notifications = { path = "crates/longhorn-notifications", version = "0.1.0" }
longhorn-tauri-operation = { path = "crates/longhorn-tauri-operation", version = "0.1.0" }
longhorn-tauri-notifications = { path = "crates/longhorn-tauri-notifications", version = "0.1.0" }
${workspaceDependencies([
  "proptest",
  "serde",
  "serde_json",
  "tauri",
  "ts-rs",
])}

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}
