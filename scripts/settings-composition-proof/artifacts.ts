import { basename, join, resolve } from "node:path";
import {
  mkdir,
  readFile,
  writeFile,
} from "node:fs/promises";

import { digest, run } from "./shared.ts";
import type {
  ArtifactIdentity,
  PoodleEvidence,
  PoodleEvidenceFile,
} from "./types.ts";

export const POODLE_ARTIFACT_SET =
  "39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74";

const poodleEvidencePath = resolve(
  process.env.POODLE_PREVIEW_EVIDENCE ??
    "../poodle/.artifacts/g12.016-A698XB/evidence.json",
);

const rustCrates = [
  "longhorn-core",
  "longhorn-config",
  "longhorn-settings",
  "longhorn-settings-config",
  "longhorn-tauri-settings",
  "longhorn-tauri-config",
] as const;

const typescriptPackages = [
  ["@longhorn/core", "core"],
  ["@longhorn/config", "config"],
  ["@longhorn/settings", "settings"],
] as const;

export async function readPoodleEvidence(): Promise<PoodleEvidence> {
  const raw = JSON.parse(
    await readFile(poodleEvidencePath, "utf8"),
  ) as PoodleEvidenceFile;
  if (raw.artifactSetId !== POODLE_ARTIFACT_SET) {
    throw new Error(`Poodle artifact set mismatch: ${raw.artifactSetId}`);
  }
  const packDirectory = join(resolve(poodleEvidencePath, ".."), "packs");
  const membership = [];
  for (const artifact of raw.artifacts) {
    const path = join(packDirectory, artifact.filename);
    const sha256 = await digest(path);
    if (sha256 !== artifact.sha256) {
      throw new Error(`${artifact.name} Poodle artifact digest mismatch`);
    }
    membership.push(`${artifact.name}:${sha256}`);
  }
  const setId = Bun.CryptoHasher.hash(
    "sha256",
    membership.join("\n"),
    "hex",
  );
  if (setId !== POODLE_ARTIFACT_SET) {
    throw new Error(`Poodle artifact membership mismatch: ${setId}`);
  }
  return { artifacts: raw.artifacts, packDirectory };
}

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
  if (name === "@longhorn/settings") {
    assertSettingsRootBoundary(
      JSON.parse(packedManifest) as {
        dependencies?: Record<string, string>;
        peerDependenciesMeta?: Record<string, { optional?: boolean }>;
      },
    );
  }
  return { name, filename: basename(path), sha256: await digest(path) };
}

function assertSettingsRootBoundary(manifest: {
  dependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
}): void {
  const dependencies = Object.keys(manifest.dependencies ?? {});
  if (
    dependencies.length !== 1 ||
    dependencies[0] !== "@longhorn/core"
  ) {
    throw new Error(
      `settings root has upward optional dependencies: ${dependencies.join(", ")}`,
    );
  }
  for (const peer of ["svelte", "@poodle/svelte"]) {
    if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) {
      throw new Error(`${peer} is not an optional settings peer`);
    }
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
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
cap-std = "4.0.2"
fs4 = { version = "1.1.0", default-features = false, features = ["sync"] }
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-config = { path = "crates/longhorn-config", version = "0.1.0" }
longhorn-settings = { path = "crates/longhorn-settings", version = "0.1.0" }
longhorn-settings-config = { path = "crates/longhorn-settings-config", version = "0.1.0" }
longhorn-tauri-settings = { path = "crates/longhorn-tauri-settings", version = "0.1.0" }
longhorn-tauri-config = { path = "crates/longhorn-tauri-config", version = "0.1.0" }
proptest = { version = "1.11.0", default-features = false, features = ["std"] }
rusqlite = { version = "=0.31.0", features = ["backup", "bundled"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
sha2 = "0.10.9"
tauri = { version = "2.10.3", default-features = false }
tempfile = "3.27.0"
ts-rs = { version = "=11.0.0", default-features = false, features = ["no-serde-warnings", "serde-compat"] }
zip = { version = "=5.1.1", default-features = false, features = ["deflate-flate2-zlib-rs"] }

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}
