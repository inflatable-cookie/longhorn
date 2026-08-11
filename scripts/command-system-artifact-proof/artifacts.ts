import { poodleRelease } from "../poodle-release.ts";
import { workspaceDependencies } from "../workspace-dependencies.ts";
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
import type { ArtifactIdentity } from "./types.ts";

// Poodle installs from the registry, so there is no evidence file to read and
// no pack to digest. poodleRelease() checks each published package's sha512
// against bun.lock and against the installed copy.
export const POODLE_RELEASE = poodleRelease();

const typescriptPackages = [
  ["@inflatable-cookie/longhorn", "longhorn"],
  ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"],
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
  // Card 164: the framework tier is one package with no dependencies and no
  // peers; everything that needed a peer moved to longhorn-poodle-svelte. The
  // command surface is proven by the entries each package exposes rather than
  // by a package per domain.
  const expectedDependencies: Record<string, readonly string[]> = {
    "@inflatable-cookie/longhorn": [],
    "@inflatable-cookie/longhorn-poodle-svelte": [],
  };
  assertExactSet(
    `${name} dependencies`,
    Object.keys(manifest.dependencies ?? {}),
    expectedDependencies[name] ?? [],
  );
  const exports = Object.keys(manifest.exports ?? {});
  if (name === "@inflatable-cookie/longhorn") {
    for (const entry of [".", "./commands", "./commands/protocol"]) {
      if (!exports.includes(entry)) {
        throw new Error(`framework package is missing the ${entry} entry`);
      }
    }
    if (Object.keys(manifest.peerDependencies ?? {}).length !== 0) {
      throw new Error("framework package declares a peer");
    }
  }
  if (name === "@inflatable-cookie/longhorn-poodle-svelte") {
    for (const entry of ["./commands/svelte", "./commands/poodle"]) {
      if (!exports.includes(entry)) {
        throw new Error(`projection package is missing the ${entry} entry`);
      }
    }
    if (
      manifest.peerDependenciesMeta?.["@inflatable-cookie/poodle-svelte"]
        ?.optional !== true
    ) {
      throw new Error("Poodle is not an optional projection peer");
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
  for (const marker of ["loophole:", "jetstream:", "soundcheck:", "split-shell:"]) {
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
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-config = { path = "crates/longhorn-config", version = "0.1.0" }
longhorn-settings = { path = "crates/longhorn-settings", version = "0.1.0" }
longhorn-command = { path = "crates/longhorn-command", version = "0.1.0" }
longhorn-command-config = { path = "crates/longhorn-command-config", version = "0.1.0" }
longhorn-command-settings = { path = "crates/longhorn-command-settings", version = "0.1.0" }
longhorn-tauri-command = { path = "crates/longhorn-tauri-command", version = "0.1.0" }
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
