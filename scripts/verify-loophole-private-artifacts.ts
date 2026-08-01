import { createHash, randomUUID } from "node:crypto";
import {
  cp,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const loopholeRoot = resolve(
  process.env.LOOPHOLE_REPO ?? resolve(repoRoot, "../loophole"),
);
const poodleRoot = resolve(repoRoot, "../poodle");
const signalRoot = resolve(repoRoot, "../signal");
const soundcheckRoot = resolve(repoRoot, "../soundcheck-library");
const poodleEvidencePath = resolve(
  process.env.POODLE_EVIDENCE_PATH ??
    resolve(
      poodleRoot,
      ".artifacts/svelte-pack-install-Rk9Mbg/evidence.json",
    ),
);

const expectedCommits = {
  loophole:
    process.env.EXPECTED_LOOPHOLE_COMMIT ??
    "4b9835ef328f13a761fe2037c45aecfb48314d21",
  poodle:
    process.env.EXPECTED_POODLE_COMMIT ??
    "208532f0d18dcd1683cdef157e370d0ba0f0d3b3",
  signal:
    process.env.EXPECTED_SIGNAL_COMMIT ??
    "4c5b60681071095aedbf9013720e76c1c89e91ed",
  soundcheck:
    process.env.EXPECTED_SOUNDCHECK_COMMIT ??
    "c84a06a598717b120efb781af679241e4d7e4f37",
} as const;
const expectedPoodleArtifactSet =
  process.env.EXPECTED_POODLE_ARTIFACT_SET ??
  "25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69";

const typescriptPackages = [
  ["@longhorn/core", "core"],
  ["@longhorn/config", "config"],
  ["@longhorn/layout", "layout"],
  ["@longhorn/surfaces", "surfaces"],
  ["@longhorn/transfer", "transfer"],
  ["@longhorn/surface-transfer", "surface-transfer"],
  ["@longhorn/settings", "settings"],
  ["@longhorn/commands", "commands"],
  ["@longhorn/history", "history"],
  ["@longhorn/tauri", "tauri"],
  ["@longhorn/svelte", "svelte"],
  ["@longhorn/poodle", "poodle"],
] as const;

const rustCrates = [
  "longhorn-core",
  "longhorn-config",
  "longhorn-tauri-config",
  "longhorn-display",
  "longhorn-windowing",
  "longhorn-tauri-windowing",
  "longhorn-windowing-config",
  "longhorn-layout",
  "longhorn-layout-config",
  "longhorn-surfaces",
  "longhorn-surfaces-config",
  "longhorn-surface-windowing",
  "longhorn-transfer",
  "longhorn-surface-transfer",
  "longhorn-tauri-transfer",
  "longhorn-settings",
  "longhorn-settings-config",
  "longhorn-tauri-settings",
  "longhorn-command",
  "longhorn-command-config",
  "longhorn-command-settings",
  "longhorn-tauri-command",
  "longhorn-history",
  "longhorn-tauri-history",
] as const;

const forbiddenTypescriptPackages = [
  "@longhorn/history-tree",
  "@longhorn/native-content",
  "@longhorn/native-content-svelte",
  "@longhorn/bridge",
  "@longhorn/operation",
  "@longhorn/notifications",
] as const;
const forbiddenRustPackages = [
  "longhorn-history-tree",
  "longhorn-tauri-history-tree",
  "longhorn-native-content",
  "longhorn-bridge",
  "longhorn-operation",
  "longhorn-notifications",
] as const;
const generatedDomains = [
  "config",
  "layout",
  "surfaces",
  "transfer",
  "surface-transfer",
  "settings",
  "commands",
  "history",
] as const;
const soundcheckSelectedPaths = [
  "Cargo.toml",
  "Cargo.lock",
  "crates/soundcheck-library-core",
  "crates/soundcheck-library-jobs",
  "crates/soundcheck-library-sqlite",
  "crates/soundcheck-library-store",
  "crates/soundcheck-library-composer",
  "crates/soundcheck-library-scan",
] as const;

interface ArtifactIdentity {
  readonly name: string;
  readonly version: string;
  readonly filename: string;
  readonly sha256: string;
}

interface PoodleEvidenceFile {
  readonly artifactSetId: string;
  readonly artifacts: readonly ArtifactIdentity[];
}

interface PackageManifest {
  readonly name: string;
  readonly version: string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  overrides?: Record<string, string>;
}

const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-loophole-private-artifact-proof-"),
);

try {
  const sources = await verifySourceReceipts();
  await verifyGeneratedBindings();
  const poodle = await readPoodleEvidence();
  const typescript = await packTypescriptArtifacts();
  const renderer = await verifyRendererConsumer(typescript.paths, poodle);
  const rust = await packAndCheckRustArtifacts();

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.loophole-private-artifact-proof.v1",
        outcome: "pass",
        sources,
        privateDependencyPlan: privateDependencyPlan(),
        poodleArtifactSet: expectedPoodleArtifactSet,
        poodleArtifacts: poodle.artifacts,
        longhornTypescriptArtifacts: typescript.identities,
        longhornRustArtifacts: rust.identities,
        generatedDomains,
        renderer,
        rustGraph: rust.graph,
        audits: {
          packageManagerPublication: false,
          registryNamesRequired: false,
          generatedBindingsDrift: false,
          producedArtifactsInstalledOutsideWorkspace: true,
          siblingSourceAliasesInProof: false,
          exactPrivateSourceReceipts: true,
          oneSvelteRuntime: true,
          onePoodleArtifactSet: true,
          fullSurfaceHierarchyPresent: true,
          forkTreePackagesAbsent: true,
          excludedLaterSystemsAbsent: true,
          donorRepositoriesWritten: false,
        },
        donorWriteAdmission: {
          admitted: true,
          nextCard: 105,
          scope: "storage and registered domains only",
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_LOOPHOLE_PRIVATE_ARTIFACT_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function verifySourceReceipts() {
  const longhornCommit = await commit(repoRoot);
  const commits = {
    loophole: await commit(loopholeRoot),
    poodle: await commit(poodleRoot),
    signal: await commit(signalRoot),
    soundcheck: await commit(soundcheckRoot),
  };
  for (const [name, expected] of Object.entries(expectedCommits)) {
    const actual = commits[name as keyof typeof commits];
    if (actual !== expected) {
      throw new Error(`${name} source commit mismatch: ${actual}`);
    }
  }

  await assertCleanRepository("Loophole", loopholeRoot);
  await assertCleanRepository("Poodle", poodleRoot);
  await assertCleanRepository("Signal", signalRoot);
  await assertSelectedSourcesClean(
    "Soundcheck Library",
    soundcheckRoot,
    soundcheckSelectedPaths,
  );
  await assertSoundcheckUnrelatedDirtyOnly();
  const selectedLonghornPaths = longhornSelectedPaths();
  await assertSelectedSourcesClean(
    "Longhorn",
    repoRoot,
    selectedLonghornPaths,
  );

  return {
    longhorn: {
      commit: longhornCommit,
      selectedTreeSha256: await treeHash(repoRoot, selectedLonghornPaths),
      selectedSourcesClean: true,
    },
    loophole: {
      commit: commits.loophole,
      pulseLockSha256: await digest(join(loopholeRoot, "pulse/Cargo.lock")),
      sparkLockSha256: await digest(join(loopholeRoot, "spark/Cargo.lock")),
      clean: true,
    },
    poodle: {
      commit: commits.poodle,
      artifactSet: expectedPoodleArtifactSet,
      clean: true,
    },
    signal: { commit: commits.signal, clean: true },
    soundcheck: {
      commit: commits.soundcheck,
      selectedTreeSha256: await treeHash(
        soundcheckRoot,
        soundcheckSelectedPaths,
      ),
      selectedSourcesClean: true,
      unrelatedRendererWorkExcluded: true,
    },
  };
}

function longhornSelectedPaths(): string[] {
  return [
    ...typescriptPackages.map(([, directory]) => `packages/${directory}`),
    ...rustCrates.map((name) => `crates/${name}`),
    "Cargo.toml",
    "Cargo.lock",
  ];
}

async function commit(root: string): Promise<string> {
  return (await run(["git", "rev-parse", "HEAD"], root)).trim();
}

async function treeHash(
  root: string,
  paths: readonly string[],
): Promise<string> {
  return hash(
    await run(["git", "ls-tree", "-r", "HEAD", "--", ...paths], root),
  );
}

async function assertCleanRepository(label: string, root: string) {
  const status = await run(["git", "status", "--porcelain"], root);
  if (status.trim()) {
    throw new Error(`${label} worktree is not clean:\n${status}`);
  }
}

async function assertSelectedSourcesClean(
  label: string,
  root: string,
  paths: readonly string[],
) {
  const status = await run(
    ["git", "status", "--porcelain", "--", ...paths],
    root,
  );
  if (status.trim()) {
    throw new Error(`${label} selected sources are not clean:\n${status}`);
  }
}

async function assertSoundcheckUnrelatedDirtyOnly() {
  const status = await run(
    ["git", "status", "--porcelain"],
    soundcheckRoot,
  );
  const unexpected = status
    .split("\n")
    .filter(Boolean)
    .filter((line) => !line.slice(3).startsWith("packages/library-svelte/"));
  if (unexpected.length > 0) {
    throw new Error(
      `Soundcheck Library has unexpected dirty paths:\n${unexpected.join("\n")}`,
    );
  }
}

async function verifyGeneratedBindings() {
  for (const domain of generatedDomains) {
    await run(
      ["cargo", "run", "-p", "longhorn-bindings", "--", domain, "check"],
      repoRoot,
    );
  }
}

async function readPoodleEvidence() {
  const raw = JSON.parse(
    await readFile(poodleEvidencePath, "utf8"),
  ) as PoodleEvidenceFile;
  if (raw.artifactSetId !== expectedPoodleArtifactSet) {
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
  if (hash(membership.join("\n")) !== expectedPoodleArtifactSet) {
    throw new Error("Poodle artifact membership mismatch");
  }
  return { artifacts: raw.artifacts, packDirectory };
}

async function packTypescriptArtifacts() {
  const artifactRoot = join(temporaryRoot, "typescript-artifacts");
  await mkdir(artifactRoot);
  const paths = new Map<string, string>();
  const identities: ArtifactIdentity[] = [];
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
    const identity = await inspectTypescriptArtifact(name, path, artifactRoot);
    paths.set(name, path);
    identities.push(identity);
  }
  return { paths, identities };
}

async function inspectTypescriptArtifact(
  name: string,
  path: string,
  artifactRoot: string,
): Promise<ArtifactIdentity> {
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
  ) as PackageManifest;
  if (manifest.name !== name || manifest.version !== "0.1.0") {
    throw new Error(`${name} packed identity mismatch`);
  }
  const serialized = JSON.stringify(manifest);
  if (serialized.includes("workspace:") || serialized.includes("link:")) {
    throw new Error(`${name} artifact contains a source alias`);
  }
  return {
    name,
    version: manifest.version,
    filename: basename(path),
    sha256: await digest(path),
  };
}

async function verifyRendererConsumer(
  longhorn: ReadonlyMap<string, string>,
  poodle: {
    readonly artifacts: readonly ArtifactIdentity[];
    readonly packDirectory: string;
  },
) {
  const stage = join(temporaryRoot, `renderer-${randomUUID()}`);
  const source = join(repoRoot, "examples/app-shell-proof/loophole");
  const common = join(repoRoot, "examples/app-shell-proof/common");
  await mkdir(stage);
  await cp(join(source, "src"), join(stage, "src"), { recursive: true });
  for (const filename of ["tsconfig.json", "setup.ts"]) {
    await cp(join(common, filename), join(stage, filename));
  }
  await writeFile(join(stage, "vitest.config.ts"), vitestConfig());
  await writeFile(
    join(stage, "src", "artifact-contract.ts"),
    artifactContract(),
  );
  await writeFile(
    join(stage, "src", "app.d.ts"),
    '/// <reference types="svelte" />\n\ndeclare module "*.css";\n',
  );

  const dependencies: Record<string, string> = {
    "@tauri-apps/api": "2.11.1",
    svelte: "5.56.8",
  };
  for (const [name, path] of longhorn) {
    dependencies[name] = fileDependency(path);
  }
  for (const artifact of poodle.artifacts) {
    dependencies[artifact.name] = fileDependency(
      join(poodle.packDirectory, artifact.filename),
    );
  }
  const manifest = {
    name: "longhorn-loophole-private-artifact-proof",
    private: true,
    type: "module",
    dependencies,
    devDependencies: {
      "@sveltejs/vite-plugin-svelte": "7.2.0",
      "@testing-library/svelte": "5.4.2",
      "@types/bun": "1.3.14",
      "happy-dom": "20.11.1",
      "svelte-check": "4.7.4",
      typescript: "6.0.3",
      vite: "8.2.0",
      vitest: "4.1.10",
    },
    overrides: dependencies,
  };
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(
    ["bun", "x", "svelte-check", "--tsconfig", "./tsconfig.json"],
    stage,
  );
  const testOutput = await run(
    ["bun", "x", "vitest", "run", "--config", "./vitest.config.ts"],
    stage,
  );

  const installed = [];
  for (const name of [
    ...longhorn.keys(),
    ...poodle.artifacts.map(({ name }) => name),
  ]) {
    installed.push(await assertArtifactInstall(stage, name));
  }
  for (const forbidden of forbiddenTypescriptPackages) {
    await assertPackageAbsent(stage, forbidden);
  }
  await assertSingleSvelteRuntime(stage);
  const svelte = await installedManifest(stage, "svelte");
  const tauri = await installedManifest(stage, "@tauri-apps/api");
  if (svelte.version !== "5.56.8") {
    throw new Error(`unexpected Svelte version ${svelte.version}`);
  }
  if (tauri.version !== "2.11.1") {
    throw new Error(`unexpected Tauri API version ${tauri.version}`);
  }
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  for (const sourceRoot of [
    resolve(repoRoot, "packages"),
    resolve(poodleRoot, "packages"),
    "workspace:",
    "link:",
  ]) {
    if (lock.includes(sourceRoot)) {
      throw new Error(`renderer lock contains source alias ${sourceRoot}`);
    }
  }
  return {
    cleanInstall: true,
    installed,
    svelte: svelte.version,
    tauriApi: tauri.version,
    typescript: "6.0.3",
    mountedTests: testCount(testOutput),
    fullSurfaceHierarchy: true,
    publicPoodleOnly: true,
    forbiddenPackagesAbsent: forbiddenTypescriptPackages,
  };
}

function artifactContract(): string {
  return `import * as config from "@longhorn/config";
import * as configPoodle from "@longhorn/config/poodle";
import * as core from "@longhorn/core";
import * as layout from "@longhorn/layout";
import * as surfaces from "@longhorn/surfaces";
import * as transfer from "@longhorn/transfer";
import * as surfaceTransfer from "@longhorn/surface-transfer";
import * as settings from "@longhorn/settings";
import * as settingsSvelte from "@longhorn/settings/svelte";
import * as settingsPoodle from "@longhorn/settings/poodle";
import * as commands from "@longhorn/commands";
import * as commandsSvelte from "@longhorn/commands/svelte";
import * as commandsPoodle from "@longhorn/commands/poodle";
import * as history from "@longhorn/history";
import * as historyTauri from "@longhorn/history/tauri";
import * as historySvelte from "@longhorn/history/svelte";
import * as historyPoodle from "@longhorn/history/poodle";
import * as tauri from "@longhorn/tauri";
import * as tauriEvents from "@longhorn/tauri/events";
import * as longhornSvelte from "@longhorn/svelte";
import * as svelteLayout from "@longhorn/svelte/layout";
import * as svelteSurfaces from "@longhorn/svelte/surfaces";
import * as svelteTransfer from "@longhorn/svelte/transfer";
import * as svelteSurfaceTransfer from "@longhorn/svelte/surface-transfer";
import * as longhornPoodle from "@longhorn/poodle";
import * as poodleBinding from "@longhorn/poodle/binding";
import * as poodleTransfer from "@longhorn/poodle/transfer";

export const exactPrivateGraph = [
  config,
  configPoodle,
  core,
  layout,
  surfaces,
  transfer,
  surfaceTransfer,
  settings,
  settingsSvelte,
  settingsPoodle,
  commands,
  commandsSvelte,
  commandsPoodle,
  history,
  historyTauri,
  historySvelte,
  historyPoodle,
  tauri,
  tauriEvents,
  longhornSvelte,
  svelteLayout,
  svelteSurfaces,
  svelteTransfer,
  svelteSurfaceTransfer,
  longhornPoodle,
  poodleBinding,
  poodleTransfer,
] as const;
`;
}

function vitestConfig(): string {
  const packages = [
    ...typescriptPackages.map(([name]) => name),
    "@poodle/headless",
    "@poodle/icons-lucide",
    "@poodle/styles",
    "@poodle/svelte",
    "@poodle/svelte-tokens",
  ];
  return `import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  ssr: { noExternal: ${JSON.stringify(packages)} },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["./setup.ts"],
  },
});
`;
}

async function assertArtifactInstall(stage: string, name: string) {
  const root = installedPackageRoot(stage, name);
  const resolved = await realpath(root);
  for (const sourceRoot of [
    resolve(repoRoot, "packages"),
    resolve(poodleRoot, "packages"),
  ]) {
    if (resolved.startsWith(`${sourceRoot}/`)) {
      throw new Error(`${name} resolved to sibling source ${resolved}`);
    }
  }
  const manifest = JSON.parse(
    await readFile(join(root, "package.json"), "utf8"),
  ) as PackageManifest;
  if (manifest.version !== "0.1.0") {
    throw new Error(`${name} resolved unexpected version ${manifest.version}`);
  }
  return { name, version: manifest.version };
}

async function assertSingleSvelteRuntime(stage: string) {
  const manifests = (await readdir(join(stage, "node_modules"), {
    recursive: true,
  })).filter(
    (path) =>
      path === "svelte/package.json" ||
      path.endsWith("/node_modules/svelte/package.json"),
  );
  if (manifests.length !== 1) {
    throw new Error(
      `expected one Svelte runtime, found ${manifests.length}: ${manifests.join(", ")}`,
    );
  }
}

async function assertPackageAbsent(stage: string, name: string) {
  try {
    await lstat(installedPackageRoot(stage, name));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the renderer graph`);
}

async function installedManifest(stage: string, name: string) {
  return JSON.parse(
    await readFile(join(installedPackageRoot(stage, name), "package.json"), "utf8"),
  ) as PackageManifest;
}

function installedPackageRoot(stage: string, name: string) {
  return join(stage, "node_modules", ...name.split("/"));
}

async function packAndCheckRustArtifacts() {
  const artifactRoot = join(temporaryRoot, "rust-artifacts");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const identities: ArtifactIdentity[] = [];
  for (const name of rustCrates) {
    const inventory = await run(
      ["cargo", "package", "-p", name, "--list", "--allow-dirty"],
      repoRoot,
    );
    if (
      !inventory.includes("Cargo.toml") ||
      !inventory.includes("src/lib.rs")
    ) {
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
      version: "0.1.0",
      filename: basename(archive),
      sha256: await digest(archive),
    });
  }
  const graph = await checkRustConsumer(workspace);
  return { identities, graph };
}

async function checkRustConsumer(workspace: string) {
  const consumer = join(workspace, "consumer");
  await mkdir(join(consumer, "src"), { recursive: true });
  await writeFile(
    join(workspace, "Cargo.toml"),
    privateRustWorkspaceManifest(),
  );
  await writeFile(
    join(consumer, "Cargo.toml"),
    `[package]
name = "longhorn-loophole-private-artifact-proof"
version = "0.0.0"
edition.workspace = true

[dependencies]
longhorn-tauri-config.workspace = true
longhorn-windowing-config.workspace = true
longhorn-layout-config.workspace = true
longhorn-surfaces-config.workspace = true
longhorn-surface-windowing.workspace = true
longhorn-tauri-transfer = { workspace = true, features = ["surface-transfer"] }
longhorn-settings-config.workspace = true
longhorn-tauri-settings.workspace = true
longhorn-command-settings.workspace = true
longhorn-tauri-command.workspace = true
longhorn-tauri-history.workspace = true
`,
  );
  await writeFile(join(consumer, "src/main.rs"), "fn main() {}\n");
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));
  await run(
    [
      "cargo",
      "+1.85.0",
      "update",
      "-p",
      "tauri",
      "--precise",
      "2.11.5",
      "--offline",
    ],
    workspace,
  );
  await run(
    [
      "cargo",
      "+1.85.0",
      "check",
      "-p",
      "longhorn-loophole-private-artifact-proof",
      "--locked",
      "--offline",
    ],
    workspace,
  );
  const tree = await run(
    [
      "cargo",
      "+1.85.0",
      "tree",
      "-p",
      "longhorn-loophole-private-artifact-proof",
      "--locked",
      "--offline",
      "--edges",
      "normal",
      "--prefix",
      "none",
    ],
    workspace,
  );
  for (const required of rustCrates) {
    if (!tree.includes(`${required} v0.1.0`)) {
      throw new Error(`Rust graph is missing ${required}`);
    }
  }
  for (const forbidden of forbiddenRustPackages) {
    if (tree.includes(`${forbidden} v`)) {
      throw new Error(`Rust graph contains forbidden package ${forbidden}`);
    }
  }
  if (!tree.includes("tauri v2.11.5")) {
    throw new Error("Rust graph did not resolve Tauri 2.11.5");
  }
  return {
    rust: "1.85",
    tauri: "2.11.5",
    cargoLockSha256: await digest(join(workspace, "Cargo.lock")),
    lockedMsrvCompatibility: true,
    offline: true,
    directDependencies: [
      "longhorn-tauri-config",
      "longhorn-windowing-config",
      "longhorn-layout-config",
      "longhorn-surfaces-config",
      "longhorn-surface-windowing",
      "longhorn-tauri-transfer/surface-transfer",
      "longhorn-settings-config",
      "longhorn-tauri-settings",
      "longhorn-command-settings",
      "longhorn-tauri-command",
      "longhorn-tauri-history",
    ],
    selectedLonghornPackages: rustCrates,
    fullSurfaceHierarchy: true,
    forbiddenPackagesAbsent: forbiddenRustPackages,
  };
}

function privateRustWorkspaceManifest(): string {
  const members = [
    "consumer",
    ...rustCrates.map((name) => `crates/${name}`),
  ];
  const internal = rustCrates
    .map((name) => `${name} = { path = "crates/${name}", version = "0.1.0" }`)
    .join("\n");
  return `[workspace]
members = ${JSON.stringify(members, null, 2)}
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
${internal}
cap-std = "4.0.2"
fs4 = { version = "1.1.0", default-features = false, features = ["sync"] }
proptest = { version = "1.11.0", default-features = false, features = ["std"] }
rusqlite = { version = "=0.31.0", features = ["backup", "bundled"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
sha2 = "0.10.9"
tauri = { version = "=2.11.5", default-features = false }
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

function privateDependencyPlan() {
  return {
    rendererBase: "loophole/aura",
    renderer: Object.fromEntries(
      typescriptPackages.map(([name, directory]) => [
        name,
        `file:../../longhorn/packages/${directory}`,
      ]),
    ),
    rustBases: ["loophole/echo", "loophole/pulse", "loophole/spark"],
    rust: Object.fromEntries(
      rustCrates.map((name) => [
        name,
        { path: `../../longhorn/crates/${name}` },
      ]),
    ),
    evidenceRule:
      "source links require exact clean commit receipts; compatibility claims come from produced artifacts",
  };
}

async function digest(path: string) {
  return hash(await readFile(path));
}

function hash(value: string | Uint8Array) {
  return createHash("sha256").update(value).digest("hex");
}

function fileDependency(path: string) {
  return `file:${path}`;
}

function testCount(output: string) {
  const match = output.match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
}

async function run(command: string[], cwd: string): Promise<string> {
  const process = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`,
    );
  }
  return `${stdout}\n${stderr}`;
}
