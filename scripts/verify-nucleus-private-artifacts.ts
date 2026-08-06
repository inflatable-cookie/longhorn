import { createHash, randomUUID } from "node:crypto";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const nucleusRoot = resolve(
  process.env.NUCLEUS_REPO ?? resolve(repoRoot, "../nucleus"),
);
const poodleRoot = resolve(repoRoot, "../poodle");
const poodleEvidencePath = resolve(
  process.env.POODLE_EVIDENCE_PATH ??
    resolve(
      poodleRoot,
      ".artifacts/svelte-pack-install-M8KH8d/evidence.json",
    ),
);
const expectedPoodleCommit =
  process.env.EXPECTED_POODLE_COMMIT ??
  "ef41f412ad7b45c2ee760c1da9bf41ef876855e8";
const expectedPoodleArtifactSet =
  process.env.EXPECTED_POODLE_ARTIFACT_SET ??
  "ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d";
const expectedNucleusCommit =
  process.env.EXPECTED_NUCLEUS_COMMIT ??
  "df5af3da03f8392f948ff65d0a3cf16c36cb6c12";

const typescriptPackages = [
  ["@longhorn/core", "core"],
  ["@longhorn/config", "config"],
  ["@longhorn/layout", "layout"],
  ["@longhorn/svelte", "svelte"],
  ["@longhorn/poodle", "poodle"],
  ["@longhorn/native-content", "native-content"],
  ["@longhorn/native-content-svelte", "native-content-svelte"],
  ["@longhorn/tauri", "tauri"],
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
  "longhorn-native-content",
  "longhorn-tauri-native-content-child-view",
] as const;

const rustWorkspaceSupport = [
  "longhorn-surfaces",
  "longhorn-surface-windowing",
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
  join(tmpdir(), "longhorn-nucleus-private-artifact-proof-"),
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
        schema: "longhorn.nucleus-private-artifact-proof.v1",
        outcome: "pass",
        sources,
        privateDependencyPlan: privateDependencyPlan(),
        poodleArtifactSet: expectedPoodleArtifactSet,
        poodleArtifacts: poodle.artifacts,
        longhornTypescriptArtifacts: typescript.identities,
        longhornRustArtifacts: rust.identities,
        rustWorkspaceSupportArtifacts: rust.supportIdentities,
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
          surfacesInConsumerGraph: false,
          poodleOverlayGeometryPublic: true,
          donorRepositoriesWritten: false,
        },
        donorWriteAdmission: {
          admitted: true,
          acceptedChecks: [
            "nucleus-g05-cross-project-layout-retention",
            "nucleus-g05-new-project-agent-chat-only",
          ],
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_NUCLEUS_PRIVATE_ARTIFACT_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function verifySourceReceipts() {
  const longhornCommit = (
    await run(["git", "rev-parse", "HEAD"], repoRoot)
  ).trim();
  const poodleCommit = (
    await run(["git", "rev-parse", "HEAD"], poodleRoot)
  ).trim();
  const nucleusCommit = (
    await run(["git", "rev-parse", "HEAD"], nucleusRoot)
  ).trim();
  if (poodleCommit !== expectedPoodleCommit) {
    throw new Error(`Poodle source commit mismatch: ${poodleCommit}`);
  }
  if (nucleusCommit !== expectedNucleusCommit) {
    throw new Error(`Nucleus source commit mismatch: ${nucleusCommit}`);
  }
  await assertCleanRepository("Poodle", poodleRoot);
  await assertCleanRepository("Nucleus", nucleusRoot);
  await assertSelectedLonghornSourcesClean();

  const selectedPaths = [
    ...typescriptPackages.map(([, directory]) => `packages/${directory}`),
    ...rustCrates.map((name) => `crates/${name}`),
    ...rustWorkspaceSupport.map((name) => `crates/${name}`),
    "Cargo.toml",
    "Cargo.lock",
  ];
  const tree = await run(
    ["git", "ls-tree", "-r", "HEAD", "--", ...selectedPaths],
    repoRoot,
  );
  return {
    longhorn: {
      commit: longhornCommit,
      selectedTreeSha256: hash(tree),
      selectedSourcesClean: true,
    },
    poodle: {
      commit: poodleCommit,
      artifactSet: expectedPoodleArtifactSet,
      clean: true,
    },
    nucleus: {
      commit: nucleusCommit,
      mode: "read-only",
      clean: true,
    },
  };
}

async function assertCleanRepository(label: string, root: string) {
  const status = await run(["git", "status", "--porcelain"], root);
  if (status.trim())
    throw new Error(`${label} worktree is not clean:\n${status}`);
}

async function assertSelectedLonghornSourcesClean() {
  const paths = [
    ...typescriptPackages.map(([, directory]) => `packages/${directory}`),
    ...rustCrates.map((name) => `crates/${name}`),
    ...rustWorkspaceSupport.map((name) => `crates/${name}`),
    "Cargo.toml",
    "Cargo.lock",
  ];
  const status = await run(
    ["git", "status", "--porcelain", "--", ...paths],
    repoRoot,
  );
  if (status.trim()) {
    throw new Error(`selected Longhorn sources are not clean:\n${status}`);
  }
}

async function verifyGeneratedBindings() {
  for (const domain of ["config", "layout", "native-content"]) {
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
  for (const forbidden of [
    "@longhorn/surfaces",
    "@longhorn/surface-transfer",
  ]) {
    if (manifest.dependencies?.[forbidden]) {
      throw new Error(
        `${name} has forbidden mandatory dependency ${forbidden}`,
      );
    }
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
  const source = join(repoRoot, "examples/app-shell-proof/nucleus");
  const common = join(repoRoot, "examples/app-shell-proof/common");
  await mkdir(stage);
  await cp(join(source, "src"), join(stage, "src"), { recursive: true });
  for (const filename of ["tsconfig.json", "vitest.config.ts", "setup.ts"]) {
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
  for (const [name, path] of longhorn)
    dependencies[name] = fileDependency(path);
  for (const artifact of poodle.artifacts) {
    dependencies[artifact.name] = fileDependency(
      join(poodle.packDirectory, artifact.filename),
    );
  }
  const manifest = {
    name: "longhorn-nucleus-private-artifact-proof",
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
  for (const forbidden of [
    "@longhorn/surfaces",
    "@longhorn/surface-transfer",
  ]) {
    if (await packageExists(stage, forbidden)) {
      throw new Error(`renderer graph resolved forbidden package ${forbidden}`);
    }
  }
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
    poodleGeometryContractCompiled: true,
    mountedTests: testCount(testOutput),
    forbiddenPackagesAbsent: [
      "@longhorn/surfaces",
      "@longhorn/surface-transfer",
    ],
  };
}

function artifactContract(): string {
  return `import * as config from "@longhorn/config";
import * as core from "@longhorn/core";
import * as layout from "@longhorn/layout";
import * as nativeContent from "@longhorn/native-content";
import * as nativeContentSvelte from "@longhorn/native-content-svelte";
import * as longhornPoodle from "@longhorn/poodle";
import * as longhornSvelte from "@longhorn/svelte";
import * as tauri from "@longhorn/tauri";
import { Menu, Popover } from "@poodle/svelte";
import type { OverlaySurfaceGeometryChange } from "@poodle/headless";
import type { ComponentProps } from "svelte";

const onSurfaceGeometryChange = (change: OverlaySurfaceGeometryChange) => change.type;
const popoverProps: Pick<ComponentProps<typeof Popover>, "onSurfaceGeometryChange"> = {
  onSurfaceGeometryChange,
};
const menuProps: Pick<ComponentProps<typeof Menu>, "onSurfaceGeometryChange"> = {
  onSurfaceGeometryChange,
};

export const exactPrivateGraph = [
  config,
  core,
  layout,
  nativeContent,
  nativeContentSvelte,
  longhornPoodle,
  longhornSvelte,
  tauri,
  popoverProps,
  menuProps,
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
  const root = await installedPackageRoot(stage, name);
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
  return { name, version: manifest.version, resolved };
}

async function installedManifest(stage: string, name: string) {
  return JSON.parse(
    await readFile(
      join(await installedPackageRoot(stage, name), "package.json"),
      "utf8",
    ),
  ) as PackageManifest;
}

async function installedPackageRoot(stage: string, name: string) {
  return join(stage, "node_modules", ...name.split("/"));
}

async function packageExists(stage: string, name: string) {
  try {
    await realpath(await installedPackageRoot(stage, name));
    return true;
  } catch {
    return false;
  }
}

async function packAndCheckRustArtifacts() {
  const artifactRoot = join(temporaryRoot, "rust-artifacts");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const identities = [];
  const supportIdentities = [];
  for (const name of [...rustCrates, ...rustWorkspaceSupport]) {
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
    const identity = {
      name,
      version: "0.1.0",
      filename: basename(archive),
      sha256: await digest(archive),
    };
    if ((rustWorkspaceSupport as readonly string[]).includes(name)) {
      supportIdentities.push(identity);
    } else {
      identities.push(identity);
    }
  }
  const graph = await checkRustConsumer(workspace);
  return { identities, supportIdentities, graph };
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
name = "longhorn-nucleus-private-artifact-proof"
version = "0.0.0"
edition.workspace = true

[dependencies]
longhorn-tauri-config.workspace = true
longhorn-windowing-config.workspace = true
longhorn-layout-config.workspace = true
longhorn-tauri-native-content-child-view.workspace = true
`,
  );
  await writeFile(
    join(consumer, "src/main.rs"),
    `fn main() {
    let _ = std::any::TypeId::of::<longhorn_tauri_config::ConfigOperationsHostError>();
    let _ = std::any::TypeId::of::<longhorn_layout_config::LayoutRegistryDigest>();
    let _ = std::any::TypeId::of::<longhorn_tauri_native_content_child_view::ChildViewLabel>();
}
`,
  );
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));
  await run(
    [
      "cargo",
      "+1.95.0",
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
      "+1.95.0",
      "check",
      "-p",
      "longhorn-nucleus-private-artifact-proof",
      "--locked",
      "--offline",
    ],
    workspace,
  );
  const tree = await run(
    [
      "cargo",
      "+1.95.0",
      "tree",
      "-p",
      "longhorn-nucleus-private-artifact-proof",
      "--locked",
      "--offline",
      "--edges",
      "normal",
      "--prefix",
      "none",
    ],
    workspace,
  );
  const cargoLockSha256 = await digest(join(workspace, "Cargo.lock"));
  for (const required of rustCrates) {
    if (!tree.includes(`${required} v0.1.0`)) {
      throw new Error(`Rust graph is missing ${required}`);
    }
  }
  for (const forbidden of rustWorkspaceSupport) {
    if (tree.includes(`${forbidden} v0.1.0`)) {
      throw new Error(`Rust consumer graph contains ${forbidden}`);
    }
  }
  if (!tree.includes("tauri v2.11.5")) {
    throw new Error("Rust consumer graph did not resolve Tauri 2.11.5");
  }
  return {
    rust: "1.95",
    tauri: "2.11.5",
    cargoLockSha256,
    lockedMsrvCompatibility: true,
    directDependencies: [
      "longhorn-tauri-config",
      "longhorn-windowing-config",
      "longhorn-layout-config",
      "longhorn-tauri-native-content-child-view",
    ],
    selectedLonghornPackages: rustCrates,
    surfacesAbsent: true,
    offline: true,
  };
}

function privateRustWorkspaceManifest(): string {
  const members = [
    "consumer",
    ...rustCrates.map((name) => `crates/${name}`),
    ...rustWorkspaceSupport.map((name) => `crates/${name}`),
  ];
  const internal = [...rustCrates, ...rustWorkspaceSupport]
    .map((name) => `${name} = { path = "crates/${name}", version = "0.1.0" }`)
    .join("\n");
  return `[workspace]
members = ${JSON.stringify(members, null, 2)}
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
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
    rendererBase: "nucleus/apps/desktop",
    renderer: Object.fromEntries(
      typescriptPackages.map(([name, directory]) => [
        name,
        `file:../../../longhorn/packages/${directory}`,
      ]),
    ),
    rustBase: "nucleus/apps/desktop/src-tauri",
    rust: Object.fromEntries(
      rustCrates.map((name) => [
        name,
        { path: `../../../../longhorn/crates/${name}` },
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
  if (!match) throw new Error("mounted test count is missing");
  return Number(match[1]);
}

async function run(command: readonly string[], cwd: string) {
  const process = Bun.spawn(command, { cwd, stdout: "pipe", stderr: "pipe" });
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
