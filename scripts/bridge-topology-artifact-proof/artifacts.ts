import { readPackageManifest } from "../proof-json.ts";
import { workspaceDependencies } from "../workspace-dependencies.ts";
import { basename, join, relative } from "node:path";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { MSRV } from "../msrv.ts";

import { assertExactSet, digest, runWithDiagnostics as run } from "../proof-support.ts";
import type { ArtifactIdentity } from "./types.ts";
import { longhornVersion } from "../longhorn-version.ts";
import { rustCommand, stagedCargoRunner } from "../rust-command.ts";
const LONGHORN_VERSION = longhornVersion();

const typescriptPackages = [
  ["@inflatable-cookie/longhorn", "longhorn"],
  ["@inflatable-cookie/longhorn-tauri", "longhorn-tauri"],
] as const;

/** Packed TypeScript package names, in pack order. */
export const packedTypescriptPackageNames: ReadonlyArray<string> = typescriptPackages.map(
  ([name]) => name,
);

/**
 * The packed package that owns the bridge surface. Its `./bridge*` exports and
 * `BRIDGE_PROTOCOL_VERSION` are checked on the packed artifact.
 */
export const bridgePackageName = "@inflatable-cookie/longhorn";

const bridgeExports = ["./bridge", "./bridge/protocol", "./bridge/stream", "./bridge/supervision"];

const rustCrates = ["longhorn-core", "longhorn-bridge", "longhorn-tauri-bridge"] as const;

const donorNames = [
  "minimal-shell",
  "ordered-streams",
  "jobs-and-service-failure",
  "capability-authority",
  "reconnecting-lifecycle",
] as const;

// Consumer repositories, whose names are product vocabulary. The roster owner
// is docs/knowledge/architecture/repo-authority-map.md. They match anywhere, so
// `name_host` and `NameHost` count.
const consumerNames = [
  "loophole",
  "nucleus",
  "soundcheck",
  "jetstream",
  "figmatic",
  "finch",
  "acowtancy",
  "bovine",
  "kimi-shell",
] as const;

const productionNetworkMarkers = [
  "reqwest",
  "tokio-tungstenite",
  "websocket",
  "eventsource",
] as const;

const credentialValueMarkers = [
  "password",
  "api_key",
  "api-key",
  "access_token",
  "refresh_token",
] as const;

// Match whole names in the source. Removing whitespace joined prose such as
// “event source” into a network marker. Match call syntax and the literal
// fixture domain prefix without joining adjacent prose.
// Tests are in scope: a crate archive holds the whole crate directory, so a
// fixture domain or a donor name in a test ships with the crate.
// Keep fixture names out of the packed crates, test modules included.
const wholeName = (marker: string) => ({
  marker,
  pattern: new RegExp(`(?<![\\w$])${marker}(?![\\w$])`, "i"),
});

/** Donor and fixture vocabulary: no packed source may carry it. */
const vocabularyPatterns = [
  ...donorNames.map(wholeName),
  ...consumerNames.map((marker) => ({ marker, pattern: new RegExp(marker, "i") })),
  { marker: "fixture.", pattern: /(?<![\w$])fixture\.(?=[\w$])/i },
];

/** Bridge sources also carry no production transport or credential value. */
const forbiddenPatterns = [
  ...vocabularyPatterns,
  ...[...productionNetworkMarkers, ...credentialValueMarkers].map(wholeName),
  { marker: "fetch(", pattern: /(?<![\w$])fetch\s*\(/i },
];

export async function packTypescriptArtifacts(
  repoRoot: string,
  artifactRoot: string,
): Promise<{
  readonly identities: ReadonlyArray<ArtifactIdentity>;
  readonly paths: ReadonlyMap<string, string>;
}> {
  const paths = new Map<string, string>();
  const identities = [];
  for (const [name, directory] of typescriptPackages) {
    await run(
      ["bun", "pm", "pack", "--destination", artifactRoot, "--ignore-scripts", "--quiet"],
      join(repoRoot, "packages", directory),
    );
    const path = join(
      artifactRoot,
      `${name.replace("@", "").replace("/", "-")}-${LONGHORN_VERSION}.tgz`,
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
  buildTarget: string,
): Promise<{
  readonly identities: ReadonlyArray<ArtifactIdentity>;
  readonly packaging: {
    readonly privateSourceInventories: true;
    readonly registryNormalizedCrates: false;
    readonly cargoPackageListPassed: true;
  };
  readonly compileGraphs: {
    readonly queryOnly: ReadonlyArray<string>;
    readonly fullHost: ReadonlyArray<string>;
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
    // Non-building and checkout-level: `package --list` reads the manifest, so
    // it runs through the switch point. The `check`/`tree` calls below lease the
    // staged workspace instead.
    const inventory = await run(
      rustCommand(["package", "-p", name, "--list", "--allow-dirty"]),
      repoRoot,
    );
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) {
      throw new Error(`${name} package inventory is incomplete`);
    }
    const archive = join(artifactRoot, `${name}-${LONGHORN_VERSION}.private.tar.gz`);
    await run(["tar", "-czf", archive, "-C", repoRoot, `crates/${name}`], repoRoot);
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({
      name,
      filename: basename(archive),
      sha256: await digest(archive),
    });
  }
  await auditRustBoundary(workspace);
  const compileGraphs = await checkRustConsumers(workspace, buildTarget);
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

async function inspectNpmArtifact(name: string, path: string, artifactRoot: string) {
  const listing = await run(["tar", "-tzf", path], artifactRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains proof or workspace material`);
  }
  const packedManifest = readPackageManifest(
    await run(["tar", "-xOzf", path, "package/package.json"], artifactRoot),
  );
  if (packedManifest.name !== name || packedManifest.version !== LONGHORN_VERSION) {
    throw new Error(`${name} packed identity mismatch`);
  }
  assertExactSet(`${name} dependencies`, Object.keys(packedManifest.dependencies ?? {}), []);
  if (
    name === "@inflatable-cookie/longhorn-tauri" &&
    packedManifest.peerDependencies?.["@tauri-apps/api"] !== "^2.10.1"
  ) {
    throw new Error("@inflatable-cookie/longhorn-tauri has an unexpected Tauri API peer");
  }

  const extractRoot = join(artifactRoot, `inspect-${name.replace("@", "").replace("/", "-")}`);
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], artifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  assertProductNeutral(`${name} TypeScript artifact`, source);
  if (name === bridgePackageName) {
    assertBridgeSurface(name, packedManifest.exports ?? {}, source);
  }
}

/**
 * Checks the packed bridge surface: the `./bridge*` exports are exactly the
 * declared set, and the packed source carries the protocol version.
 */
export function assertBridgeSurface(
  name: string,
  exportsMap: Readonly<Record<string, unknown>>,
  source: string,
): void {
  const bridgeKeys = Object.keys(exportsMap).filter(
    (key) => key === "./bridge" || key.startsWith("./bridge/"),
  );
  assertExactSet(`${name} bridge exports`, bridgeKeys, bridgeExports);
  if (!source.includes("BRIDGE_PROTOCOL_VERSION = 1")) {
    throw new Error(`${name} packed bridge protocol version is missing`);
  }
}

/**
 * Checks every file each packed crate carries, not only `src`. `longhorn-core`
 * owns credential types, whose field names are not values, so it is checked for
 * donor and fixture vocabulary only.
 */
export async function auditRustBoundary(workspace: string): Promise<void> {
  for (const name of rustCrates) {
    const patterns = name === "longhorn-core" ? vocabularyPatterns : forbiddenPatterns;
    const crateRoot = join(workspace, "crates", name);
    const entries = await readdir(crateRoot, { recursive: true, withFileTypes: true });
    for (const entry of entries.filter((candidate) => candidate.isFile())) {
      const path = join(entry.parentPath, entry.name);
      assertProductNeutral(
        `Rust bridge artifact ${name}/${relative(crateRoot, path)}`,
        await readFile(path, "utf8"),
        patterns,
      );
    }
  }
}

export function assertProductNeutral(
  label: string,
  source: string,
  patterns: ReadonlyArray<{
    readonly marker: string;
    readonly pattern: RegExp;
  }> = forbiddenPatterns,
): void {
  for (const { marker, pattern } of patterns) {
    if (pattern.test(source)) {
      throw new Error(`${label} contains forbidden marker: ${marker}`);
    }
  }
}

async function readSourceTree(root: string): Promise<string> {
  const files = (await readdir(root, { recursive: true }))
    .filter((path) => /\.(rs|ts|svelte)$/.test(path))
    .sort();
  return (await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))).join("\n");
}

async function checkRustConsumers(
  workspace: string,
  buildTarget: string,
): Promise<{
  readonly queryOnly: ReadonlyArray<string>;
  readonly fullHost: ReadonlyArray<string>;
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

  const staged = await stagedCargoRunner(workspace, { target: buildTarget });
  await run(
    staged(["cargo", "check", "-p", "longhorn-bridge-query-artifact-proof", "--offline"]),
    workspace,
  );
  await run(
    staged(["cargo", "check", "-p", "longhorn-bridge-full-artifact-proof", "--offline"]),
    workspace,
  );
  const queryTree = await run(
    staged([
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-query-artifact-proof",
      "--offline",
      "--depth",
      "1",
      "--prefix",
      "none",
    ]),
    workspace,
  );
  const queryFeatures = await run(
    staged([
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-query-artifact-proof",
      "--offline",
      "--edges",
      "features",
    ]),
    workspace,
  );
  const fullTree = await run(
    staged([
      "cargo",
      "tree",
      "-p",
      "longhorn-bridge-full-artifact-proof",
      "--offline",
      "--depth",
      "1",
      "--prefix",
      "none",
    ]),
    workspace,
  );
  if (
    queryTree.includes("longhorn-tauri-bridge") ||
    queryFeatures.includes('longhorn-bridge feature "supervision"')
  ) {
    throw new Error("query-only Rust graph acquired an optional bridge edge");
  }
  if (!fullTree.includes("longhorn-tauri-bridge") || !fullTree.includes("longhorn-bridge")) {
    throw new Error("full Rust graph is missing a selected bridge edge");
  }
  return {
    queryOnly: directPackages(queryTree),
    fullHost: directPackages(fullTree),
    supervisionOptional: true,
    tauriOptional: true,
  };
}

function directPackages(tree: string): ReadonlyArray<string> {
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
version = "${LONGHORN_VERSION}"
edition = "2024"
rust-version = "${MSRV}"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
longhorn-core = { path = "crates/longhorn-core", version = "${LONGHORN_VERSION}" }
longhorn-bridge = { path = "crates/longhorn-bridge", version = "${LONGHORN_VERSION}", default-features = false }
longhorn-tauri-bridge = { path = "crates/longhorn-tauri-bridge", version = "${LONGHORN_VERSION}" }
${workspaceDependencies(["proptest", "serde", "serde_json", "tauri", "ts-rs"])}

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}
