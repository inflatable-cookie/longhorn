import { poodleRelease } from "../poodle-release.ts";
import { workspaceDependencies } from "../workspace-dependencies.ts";
import { homedir } from "node:os";
import { basename, join, resolve } from "node:path";
import {
  mkdir,
  readFile,
  writeFile,
} from "node:fs/promises";
import { MSRV, MSRV_TOOLCHAIN } from "../msrv.ts";

import { digest, run } from "./shared.ts";
import type { ArtifactIdentity } from "./types.ts";

// Poodle installs from the registry, so there is no evidence file to read and
// no pack to digest. poodleRelease() checks each published package's sha512
// against bun.lock and against the installed copy.
export const POODLE_RELEASE = poodleRelease();

const rustCrates = [
  "longhorn-core",
  "longhorn-config",
  "longhorn-settings",
  "longhorn-settings-config",
  "longhorn-tauri-settings",
  "longhorn-tauri-config",
] as const;

const typescriptPackages = [
  ["@inflatable-cookie/longhorn", "longhorn"],
  ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"],
] as const;

export async function packTypescriptArtifacts(
  repoRoot: string,
  artifactRoot: string,
): Promise<{
  readonly identities: readonly ArtifactIdentity[];
  readonly paths: ReadonlyMap<string, string>;
}> {
  const paths = new Map<string, string>();
  const identities = [];
  for (const [name, directory] of typescriptPackages) {
    await run(
      [
        "bun",
        "pm",
        "pack",
        "--destination",
        artifactRoot,
        "--ignore-scripts",
        "--quiet",
      ],
      join(repoRoot, "packages", directory),
    );
    const path = join(
      artifactRoot,
      `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`,
    );
    paths.set(name, path);
    identities.push(await inspectNpmArtifact(name, path, repoRoot));
  }
  return { identities, paths };
}

export async function packAndCheckRustArtifacts(
  repoRoot: string,
  temporaryRoot: string,
): Promise<readonly ArtifactIdentity[]> {
  const artifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const identities = [];
  for (const name of rustCrates) {
    await run(["cargo", "package", "-p", name, "--list", "--allow-dirty"], repoRoot);
    const archive = join(
      artifactRoot,
      `${name}-0.1.0.private.tar.gz`,
    );
    await run(
      [
        "tar",
        "-czf",
        archive,
        "-C",
        repoRoot,
        `crates/${name}`,
      ],
      repoRoot,
    );
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({
      name,
      filename: basename(archive),
      sha256: await digest(archive),
    });
  }
  await checkRustConsumer(workspace);
  return identities;
}

async function inspectNpmArtifact(
  name: string,
  path: string,
  repoRoot: string,
): Promise<ArtifactIdentity> {
  const listing = await run(["tar", "-tzf", path], repoRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains proof or workspace material`);
  }
  const packedManifest = await run(
    ["tar", "-xOzf", path, "package/package.json"],
    repoRoot,
  );
  if (
    packedManifest.includes("workspace:") ||
    packedManifest.includes("link:")
  ) {
    throw new Error(`${name} artifact contains workspace dependency aliases`);
  }
  if (name === "@inflatable-cookie/longhorn") {
    assertFrameworkRootBoundary(
      JSON.parse(packedManifest) as {
        dependencies?: Record<string, string>;
        peerDependenciesMeta?: Record<string, { optional?: boolean }>;
      },
    );
  }
  return { name, filename: basename(path), sha256: await digest(path) };
}

function assertFrameworkRootBoundary(manifest: {
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
}): void {
  // Card 164 made this claim stronger, not weaker. The settings root was
  // allowed exactly one dependency, on longhorn-core; the consolidated
  // framework package is allowed none at all and no peers, because everything
  // that needed a peer moved to longhorn-poodle-svelte.
  const dependencies = Object.keys(manifest.dependencies ?? {});
  if (dependencies.length !== 0) {
    throw new Error(
      `framework root declares dependencies: ${dependencies.join(", ")}`,
    );
  }
  const peers = Object.keys(manifest.peerDependencies ?? {});
  if (peers.length !== 0) {
    throw new Error(`framework root declares peers: ${peers.join(", ")}`);
  }
}

async function checkRustConsumer(
  workspace: string,
): Promise<void> {
  const consumer = join(workspace, "consumer");
  await mkdir(join(consumer, "src"), { recursive: true });
  await writeFile(
    join(workspace, "Cargo.toml"),
    privateWorkspaceManifest(),
  );
  await writeFile(
    join(consumer, "Cargo.toml"),
    `[package]
name = "longhorn-settings-artifact-proof"
version = "0.0.0"
edition.workspace = true

[dependencies]
longhorn-settings-config.workspace = true
longhorn-tauri-settings.workspace = true
longhorn-tauri-config.workspace = true
`,
  );
  await writeFile(
    join(consumer, "src/main.rs"),
    `fn main() {
    let _ = longhorn_settings_config::STORAGE_SETTINGS_PAGE_ID;
    let _ = longhorn_tauri_settings::SETTINGS_REGISTRY_CHANGED_EVENT;
    let _ = std::any::TypeId::of::<longhorn_tauri_config::ConfigOperationsHostError>();
}
`,
  );
  await run(
    [
      "cargo",
      "check",
      "-p",
      "longhorn-settings-artifact-proof",
      "--offline",
    ],
    workspace,
  );
}

function privateWorkspaceManifest(): string {
  return `[workspace]
members = [
  "consumer",
  "crates/longhorn-core",
  "crates/longhorn-config",
  "crates/longhorn-settings",
  "crates/longhorn-settings-config",
  "crates/longhorn-tauri-settings",
  "crates/longhorn-tauri-config",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "${MSRV}"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-config = { path = "crates/longhorn-config", version = "0.1.0" }
longhorn-settings = { path = "crates/longhorn-settings", version = "0.1.0" }
longhorn-settings-config = { path = "crates/longhorn-settings-config", version = "0.1.0" }
longhorn-tauri-settings = { path = "crates/longhorn-tauri-settings", version = "0.1.0" }
longhorn-tauri-config = { path = "crates/longhorn-tauri-config", version = "0.1.0" }
${workspaceDependencies([
  "cap-std",
  "fs4",
  "proptest",
  "rusqlite",
  "serde",
  "serde_json",
  "sha2",
  "tauri",
  "tempfile",
  "ts-rs",
  "zip",
])}

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}


/**
 * Whether the proof may install Poodle from the sibling checkout.
 *
 * Off unless asked for. The proof exists to show that a consumer can install
 * what a consumer resolves, and the registry is what a consumer resolves --
 * so this weakens it, and every run that uses it says so in its artifact.
 *
 * It exists because a Longhorn change can depend on an unreleased Poodle
 * component, and the alternative is a red gate for as long as the release
 * takes. A red gate hides the next real failure, which costs more than a
 * recorded exemption. Remove the flag from `effigy.toml` once Poodle ships.
 */
export const ACCEPT_LINKED_POODLE =
  process.env.LONGHORN_PROOF_ACCEPT_LINKED_POODLE === "1";

const POODLE_PACKAGE_DIRECTORIES = new Map([
  ["@inflatable-cookie/poodle-core", "packages/core"],
  ["@inflatable-cookie/poodle-svelte", "packages/svelte/components"],
]);

/**
 * Packs Poodle from the sibling checkout into installable tarballs.
 *
 * Packed rather than linked on purpose. The consumer still performs a clean
 * install of real bytes with no symlink and no source alias, so every claim
 * the proof makes survives except one: the bytes came from a local pack rather
 * than from the registry. That is the single thing the exemption gives up, and
 * naming it here is cheaper than discovering it later.
 */
export async function packLinkedPoodle(
  artifactRoot: string,
): Promise<ReadonlyMap<string, string>> {
  const paths = new Map<string, string>();
  const checkout = resolve(homedir(), "Dev/projects/poodle");
  for (const [name, directory] of POODLE_PACKAGE_DIRECTORIES) {
    await run(
      ["bun", "pm", "pack", "--destination", artifactRoot, "--ignore-scripts", "--quiet"],
      join(checkout, directory),
    );
    paths.set(
      name,
      join(artifactRoot, `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`),
    );
  }
  return paths;
}
