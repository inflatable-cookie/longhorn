import { workspaceDependencies } from "../workspace-dependencies.ts";
import { basename, join } from "node:path";
import {
  mkdir,
  readFile,
  readdir,
  writeFile,
} from "node:fs/promises";

import { assertExactSet, digest, run } from "./shared.ts";
import type { ArtifactIdentity } from "./types.ts";

const typescriptPackages = [["@inflatable-cookie/longhorn", "longhorn"], ["@inflatable-cookie/longhorn-tauri", "longhorn-tauri"]] as const;

const rustCrates = [
  "longhorn-core",
  "longhorn-bridge",
  "longhorn-tauri-bridge",
] as const;

const donorNames = [
  "split-shell",
  "ordered-streams",
  "jobs-and-service-failure",
  "capability-authority",
  "reconnecting-lifecycle",
] as const;

const productionNetworkMarkers = [
  "reqwest",
  "tokio-tungstenite",
  "websocket",
  "eventsource",
  "fetch(",
  "new websocket",
] as const;

const credentialValueMarkers = [
  "password",
  "api_key",
  "api-key",
  "access_token",
  "refresh_token",
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
  readonly compileGraphs: {
    readonly queryOnly: readonly string[];
    readonly fullHost: readonly string[];
    readonly supervisionOptional: true;
    readonly tauriOptional: true;
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
  const compileGraphs = await checkRustConsumers(workspace);
  return {
    identities,
    packaging: {
      privateSourceInventories: true,
      registryNormalizedCrates: false,
      cargoPackageListPassed: true,
    },
    compileGraphs,
  };
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
  const packedManifest = JSON.parse(
    await run(["tar", "-xOzf", path, "package/package.json"], artifactRoot),
  ) as {
    readonly name: string;
    readonly version: string;
    readonly dependencies?: Record<string, string>;
    readonly peerDependencies?: Record<string, string>;
    readonly exports?: Record<string, unknown>;
  };
  if (packedManifest.name !== name || packedManifest.version !== "0.1.0") {
    throw new Error(`${name} packed identity mismatch`);
  }
  const expectedDependencies: Record<string, readonly string[]> = {
    "@inflatable-cookie/longhorn": [],
    "@inflatable-cookie/longhorn-poodle-svelte": [],
    "@inflatable-cookie/longhorn-tauri": [],
  };
  assertExactSet(
    `${name} dependencies`,
    Object.keys(packedManifest.dependencies ?? {}),
    expectedDependencies[name] ?? [],
  );
  if (
    name === "@inflatable-cookie/longhorn-bridge" &&
    packedManifest.peerDependencies?.["@inflatable-cookie/longhorn-tauri"] !== "0.1.0"
  ) {
    throw new Error(
      "@inflatable-cookie/longhorn-bridge must keep @inflatable-cookie/longhorn-tauri as an optional peer",
    );
  }
  if (
    name === "@inflatable-cookie/longhorn-tauri" &&
    packedManifest.peerDependencies?.["@tauri-apps/api"] !== "^2.10.1"
  ) {
    throw new Error("@inflatable-cookie/longhorn-tauri has an unexpected Tauri API peer");
  }
  if (name === "@inflatable-cookie/longhorn-bridge") {
    assertExactSet(
      "@inflatable-cookie/longhorn-bridge exports",
      Object.keys(packedManifest.exports ?? {}),
      [
        ".",
        "./protocol",
        "./stream",
        "./supervision",
        "./tauri",
        "./tauri-events",
        "./package.json",
      ],
    );
  }

  const extractRoot = join(
    artifactRoot,
    `inspect-${name.replace("@", "").replace("/", "-")}`,
  );
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], artifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  assertProductNeutral(`${name} TypeScript artifact`, source);
  if (
    name === "@inflatable-cookie/longhorn-bridge" &&
    !source.includes("BRIDGE_PROTOCOL_VERSION = 1")
  ) {
    throw new Error("packed bridge protocol version is missing");
  }
}

async function auditRustBoundary(workspace: string): Promise<void> {
  const source = (
    await Promise.all(
      ["longhorn-bridge", "longhorn-tauri-bridge"].map(async (name) => {
        const crateRoot = join(workspace, "crates", name);
        return `${await readFile(join(crateRoot, "Cargo.toml"), "utf8")}\n${
          await readSourceTree(join(crateRoot, "src"))
        }`;
      }),
    )
  ).join("\n");
  assertProductNeutral("Rust bridge artifacts", source);
}

function assertProductNeutral(label: string, source: string): void {
  const normalized = source.toLowerCase().replaceAll(/\s+/g, "");
  for (const marker of [
    ...donorNames,
    ...productionNetworkMarkers,
    ...credentialValueMarkers,
    "fixture.",
  ]) {
    if (normalized.includes(marker.replaceAll(/\s+/g, ""))) {
      throw new Error(`${label} contains forbidden marker: ${marker}`);
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

async function checkRustConsumers(workspace: string): Promise<{
  readonly queryOnly: readonly string[];
  readonly fullHost: readonly string[];
  readonly supervisionOptional: true;
  readonly tauriOptional: true;
}> {
  await mkdir(join(workspace, "query-consumer", "src"), { recursive: true });
  await mkdir(join(workspace, "full-consumer", "src"), { recursive: true });
  await writeFile(join(workspace, "Cargo.toml"), privateWorkspaceManifest());
  await writeFile(
    join(workspace, "query-consumer", "Cargo.toml"),
    consumerManifest("longhorn-bridge-query-artifact-proof", false),
  );
  await writeFile(
    join(workspace, "query-consumer", "src", "main.rs"),
    `fn main() {
    let _ = longhorn_bridge::BRIDGE_PROTOCOL_VERSION;
    let _ = std::any::TypeId::of::<longhorn_bridge::BridgeHostForm>();
}
`,
  );
  await writeFile(
    join(workspace, "full-consumer", "Cargo.toml"),
    consumerManifest("longhorn-bridge-full-artifact-proof", true),
  );
  await writeFile(
    join(workspace, "full-consumer", "src", "main.rs"),
    `fn main() {
    let _ = longhorn_bridge::BridgeServiceMachine::new(
        longhorn_bridge::BridgeServiceOwnership::OwnedLocal,
    );
    let _ = longhorn_tauri_bridge::BRIDGE_DOMAIN_EVENT;
}
`,
  );

  await run(
    ["cargo", "check", "-p", "longhorn-bridge-query-artifact-proof", "--offline"],
    workspace,
  );
  await run(
    ["cargo", "check", "-p", "longhorn-bridge-full-artifact-proof", "--offline"],
    workspace,
  );
  const queryTree = await run(
    [
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-query-artifact-proof",
      "--offline",
      "--depth",
      "1",
      "--prefix",
      "none",
    ],
    workspace,
  );
  const queryFeatures = await run(
    [
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-query-artifact-proof",
      "--offline",
      "--edges",
      "features",
    ],
    workspace,
  );
  const fullTree = await run(
    [
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-full-artifact-proof",
      "--offline",
      "--depth",
      "1",
      "--prefix",
      "none",
    ],
    workspace,
  );
  if (
    queryTree.includes("longhorn-tauri-bridge") ||
    queryFeatures.includes('longhorn-bridge feature "supervision"')
  ) {
    throw new Error("query-only Rust graph acquired an optional bridge edge");
  }
  if (
    !fullTree.includes("longhorn-tauri-bridge") ||
    !fullTree.includes("longhorn-bridge")
  ) {
    throw new Error("full Rust graph is missing a selected bridge edge");
  }
  return {
    queryOnly: directPackages(queryTree),
    fullHost: directPackages(fullTree),
    supervisionOptional: true,
    tauriOptional: true,
  };
}

function directPackages(tree: string): readonly string[] {
  return tree
    .split("\n")
    .map((line) => line.trim().split(/\s+/)[0])
    .filter((name): name is string => Boolean(name))
    .filter((name) => !name.includes("artifact-proof"));
}

function consumerManifest(name: string, full: boolean): string {
  return `[package]
name = "${name}"
version = "0.0.0"
edition.workspace = true

[dependencies]
longhorn-bridge = { workspace = true${full ? ', features = ["supervision"]' : ""} }
${full ? "longhorn-tauri-bridge.workspace = true" : ""}
`;
}

function privateWorkspaceManifest(): string {
  return `[workspace]
members = [
  "query-consumer",
  "full-consumer",
  "crates/longhorn-core",
  "crates/longhorn-bridge",
  "crates/longhorn-tauri-bridge",
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
longhorn-bridge = { path = "crates/longhorn-bridge", version = "0.1.0", default-features = false }
longhorn-tauri-bridge = { path = "crates/longhorn-tauri-bridge", version = "0.1.0" }
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
