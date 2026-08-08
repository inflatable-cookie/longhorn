import { basename, join, resolve } from "node:path";
import {
  cp,
  mkdir,
  readFile,
  readdir,
  writeFile,
} from "node:fs/promises";

import {
  assertExactSet,
  digest,
  parseTrace,
  run,
} from "./shared.ts";
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

const typescriptPackages = [
  ["@inflatable-cookie/longhorn-core", "core"],
  ["@inflatable-cookie/longhorn-settings", "settings"],
  ["@inflatable-cookie/longhorn-commands", "commands"],
] as const;

const rustCrates = [
  "longhorn-core",
  "longhorn-config",
  "longhorn-settings",
  "longhorn-command",
  "longhorn-command-config",
  "longhorn-command-settings",
  "longhorn-tauri-command",
] as const;

const optionalRustEdges = [
  "longhorn-command-config",
  "longhorn-config",
  "longhorn-settings",
  "longhorn-command-settings",
  "longhorn-tauri-command",
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
    await inspectNpmArtifact(name, path, artifactRoot);
    paths.set(name, path);
    identities.push({
      name,
      filename: basename(path),
      sha256: await digest(path),
    });
  }
  return { identities, paths };
}

async function inspectNpmArtifact(
  name: string,
  path: string,
  artifactRoot: string,
): Promise<void> {
  const listing = await run(["tar", "-tzf", path], artifactRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains proof or workspace material`);
  }
  const manifest = JSON.parse(
    await run(["tar", "-xOzf", path, "package/package.json"], artifactRoot),
  ) as {
    readonly name: string;
    readonly version: string;
    readonly dependencies?: Record<string, string>;
    readonly peerDependencies?: Record<string, string>;
    readonly peerDependenciesMeta?: Record<string, { optional?: boolean }>;
    readonly exports?: Record<string, unknown>;
  };
  if (manifest.name !== name || manifest.version !== "0.1.0") {
    throw new Error(`${name} packed identity mismatch`);
  }
  const expectedDependencies: Record<string, readonly string[]> = {
    "@inflatable-cookie/longhorn-core": [],
    "@inflatable-cookie/longhorn-settings": ["@inflatable-cookie/longhorn-core"],
    "@inflatable-cookie/longhorn-commands": [],
  };
  assertExactSet(
    `${name} dependencies`,
    Object.keys(manifest.dependencies ?? {}),
    expectedDependencies[name]!,
  );
  if (name === "@inflatable-cookie/longhorn-commands") {
    assertExactSet(
      "command package exports",
      Object.keys(manifest.exports ?? {}),
      [".", "./protocol", "./svelte", "./poodle", "./package.json"],
    );
    for (const peer of ["svelte", "@inflatable-cookie/poodle-svelte"]) {
      if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) {
        throw new Error(`${peer} is not an optional command peer`);
      }
    }
  }
  const extractRoot = join(
    artifactRoot,
    `inspect-${name.replace("@", "").replace("/", "-")}`,
  );
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], artifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  assertSharedSourceNeutral(`${name} TypeScript artifact`, source);
}

export async function packAndCheckRustArtifacts(
  repoRoot: string,
  temporaryRoot: string,
): Promise<{
  readonly identities: readonly ArtifactIdentity[];
  readonly packaging: {
    readonly privateSourceInventories: true;
    readonly registryNormalizedCrates: false;
    readonly cargoPackageListPassed: true;
  };
  readonly graphs: {
    readonly jetstream: readonly string[];
    readonly loophole: readonly string[];
  };
  readonly traces: {
    readonly jetstream: Record<string, unknown>;
    readonly loophole: Record<string, unknown>;
  };
}> {
  const artifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const identities = [];
  for (const name of rustCrates) {
    const inventory = await run(
      ["cargo", "package", "-p", name, "--list", "--allow-dirty"],
      repoRoot,
    );
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) {
      throw new Error(`${name} package inventory is incomplete`);
    }
    const archive = join(artifactRoot, `${name}-0.1.0.private.tar.gz`);
    await run(
      ["tar", "-czf", archive, "-C", repoRoot, `crates/${name}`],
      repoRoot,
    );
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({
      name,
      filename: basename(archive),
      sha256: await digest(archive),
    });
  }
  await auditRustBoundary(workspace);
  const consumersRoot = join(workspace, "consumers");
  await mkdir(consumersRoot);
  for (const shape of ["jetstream", "loophole"] as const) {
    await cp(
      join(repoRoot, "examples", "command-system-proof", "rust", shape),
      join(consumersRoot, shape),
      { recursive: true },
    );
  }
  await writeFile(join(workspace, "Cargo.toml"), privateWorkspaceManifest());
  const traces = {
    jetstream: parseTrace(
      await run(
        [
          "cargo",
          "run",
          "-p",
          "longhorn-jetstream-command-artifact-proof",
          "--offline",
          "--quiet",
        ],
        workspace,
      ),
    ),
    loophole: parseTrace(
      await run(
        [
          "cargo",
          "run",
          "-p",
          "longhorn-loophole-command-artifact-proof",
          "--offline",
          "--quiet",
        ],
        workspace,
      ),
    ),
  };
  const jetstreamTree = await run(
    [
      "cargo",
      "tree",
      "-p",
      "longhorn-jetstream-command-artifact-proof",
      "--offline",
      "--prefix",
      "none",
    ],
    workspace,
  );
  const loopholeTree = await run(
    [
      "cargo",
      "tree",
      "-p",
      "longhorn-loophole-command-artifact-proof",
      "--offline",
      "--prefix",
      "none",
    ],
    workspace,
  );
  for (const edge of optionalRustEdges) {
    if (jetstreamTree.includes(edge)) {
      throw new Error(`Jetstream Rust graph acquired optional edge ${edge}`);
    }
    if (!loopholeTree.includes(edge)) {
      throw new Error(`Loophole Rust graph is missing selected edge ${edge}`);
    }
  }
  return {
    identities,
    packaging: {
      privateSourceInventories: true,
      registryNormalizedCrates: false,
      cargoPackageListPassed: true,
    },
    graphs: {
      jetstream: longhornPackages(jetstreamTree),
      loophole: longhornPackages(loopholeTree),
    },
    traces,
  };
}

async function auditRustBoundary(workspace: string): Promise<void> {
  const source = (
    await Promise.all(
      [
        "longhorn-command",
        "longhorn-command-config",
        "longhorn-command-settings",
        "longhorn-tauri-command",
      ].map(async (name) =>
        readSourceTree(join(workspace, "crates", name, "src"))
      ),
    )
  ).join("\n");
  assertSharedSourceNeutral("Rust command artifacts", source);
  const normalized = source.toLowerCase().replaceAll(/\s+/g, "");
  for (const marker of [
    "longhorn_command_execute",
    "genericcommandbus",
    "invoke(command_id",
    "invoke(commandid",
  ]) {
    if (normalized.includes(marker)) {
      throw new Error(`Rust command artifacts contain execution bus ${marker}`);
    }
  }
}

function assertSharedSourceNeutral(label: string, source: string): void {
  const normalized = source.toLowerCase().replaceAll(/\s+/g, "");
  for (const marker of ["loophole:", "jetstream:", "soundcheck:", "bovine:"]) {
    if (normalized.includes(marker)) {
      throw new Error(`${label} contains donor payload marker ${marker}`);
    }
  }
}

async function readSourceTree(root: string): Promise<string> {
  const files = (await readdir(root, { recursive: true }))
    .filter((path) => /\.(rs|ts|svelte)$/.test(path))
    .sort();
  return (
    await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))
  ).join("\n");
}

function longhornPackages(tree: string): readonly string[] {
  return [
    ...new Set(
      tree
        .split("\n")
        .map((line) => line.trim().split(/\s+/)[0] ?? "")
        .filter((name) => name.startsWith("longhorn-"))
        .filter((name) => !name.endsWith("-artifact-proof")),
    ),
  ].sort();
}

function privateWorkspaceManifest(): string {
  return `[workspace]
members = [
  "consumers/jetstream",
  "consumers/loophole",
  "crates/longhorn-core",
  "crates/longhorn-config",
  "crates/longhorn-settings",
  "crates/longhorn-command",
  "crates/longhorn-command-config",
  "crates/longhorn-command-settings",
  "crates/longhorn-tauri-command",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
cap-std = "4.0.2"
fs4 = { version = "1.1.0", default-features = false, features = ["sync"] }
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-config = { path = "crates/longhorn-config", version = "0.1.0" }
longhorn-settings = { path = "crates/longhorn-settings", version = "0.1.0" }
longhorn-command = { path = "crates/longhorn-command", version = "0.1.0" }
longhorn-command-config = { path = "crates/longhorn-command-config", version = "0.1.0" }
longhorn-command-settings = { path = "crates/longhorn-command-settings", version = "0.1.0" }
longhorn-tauri-command = { path = "crates/longhorn-tauri-command", version = "0.1.0" }
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
