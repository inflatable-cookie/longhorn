import { requireRecord } from "./proof-json.ts";
import { errorCode } from "./proof-support.ts";
import { readPackageIdentity, readPackageManifest, readPermissions } from "./proof-json.ts";
import { requireValue } from "./proof-support.ts";
import {
  parseTrace,
  digest,
  fileDependency,
  assertExactSet,
  runArtifactCommand as run,
} from "./proof-support.ts";
import { installProofStage } from "./proof-install.ts";
import { poodleRelease } from "./poodle-release.ts";
import { workspaceDependencies } from "./workspace-dependencies.ts";
import { randomUUID } from "node:crypto";
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
import { MSRV, MSRV_TOOLCHAIN } from "./msrv.ts";
import { longhornVersion } from "./longhorn-version.ts";
import {
  prepareStagedWorkspace,
  privateCargoCommand,
  rustCommand,
  rustExec,
  taskScratch,
} from "./rust-command.ts";

// Poodle installs from the registry; poodleRelease() checks each published
// package's sha512 against bun.lock and against the installed copy.
const POODLE_RELEASE = poodleRelease();
// Longhorn's own coordinated version. Poodle carries its own, released
// separately, so the two cannot be one literal any more.
const LONGHORN_VERSION = longhornVersion();
const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "history-tree-artifact-proof");
const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-history-tree-artifact-proof-"));
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(typescriptArtifactRoot);
// Private final Cargo outputs for the staged workspace, on external build
// scratch. The intermediates keep the build directory each exec lease supplies.
const buildScratch = await taskScratch("longhorn-history-tree-rust");

type Shape = "document" | "loophole";
type Json = Record<string, unknown>;
interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}
const policies = {
  document: {
    rust: ["longhorn-core", "longhorn-history", "longhorn-history-tree"],
    longhorn: ["@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn/history-tree"],
    permissions: [],
    forbidden: ["@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn-tauri"],
    metrics: { nodes: 132, branches: 5, payload: 4224, lca: 65, baseline: 99_295 },
  },
  loophole: {
    rust: [
      "longhorn-core",
      "longhorn-history",
      "longhorn-history-tree",
      "longhorn-tauri-history-tree",
    ],
    longhorn: [
      "@inflatable-cookie/longhorn-poodle-svelte",
      "@inflatable-cookie/longhorn-tauri",
      "@inflatable-cookie/longhorn",
    ],
    imports: [
      "@inflatable-cookie/longhorn-poodle-svelte/history-tree/svelte",
      "@inflatable-cookie/longhorn-tauri/history-tree",
      "@inflatable-cookie/longhorn/history-tree",
    ],
    permissions: [
      "allow-longhorn-history-tree-read",
      "allow-longhorn-history-tree-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn/history"],
    metrics: { nodes: 2112, branches: 65, payload: 540_672, lca: 1025, baseline: 7_534_856 },
  },
} as const;

try {
  // Checkout-level and non-building: the bindings check runs through the switch
  // point and pools. Only the staged-workspace builds below keep private
  // outputs.
  await run(
    rustCommand(["run", "-p", "longhorn-bindings", "--", "history-tree", "check"]),
    repoRoot,
  );
  const typescript = await packTypescriptArtifacts();
  const rust = await packAndRunRustArtifacts();
  const consumers = [];
  for (const shape of ["document", "loophole"] as const) {
    consumers.push(await verifyTypescriptConsumer(shape, typescript.paths, rust.traces[shape]));
  }
  const rendererFixturesPayloadFree = (["document", "loophole"] as const).every(
    (shape) => !containsKey(rust.traces[shape].rendererFixture, "payload"),
  );
  if (!rendererFixturesPayloadFree) {
    throw new Error("renderer fixture contains product payload");
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.history-tree-artifact-proof.v1",
        linearControl: "effigy proof:history-system-artifacts",
        poodleVersion: POODLE_RELEASE.version,
        rustArtifacts: rust.identities,
        rustGraphs: rust.graphs,
        artifactCoreTestCount: rust.testCount,
        nativeTraces: Object.fromEntries(
          (["document", "loophole"] as const).map((shape) => [
            shape,
            {
              publicTrace: rust.traces[shape].publicTrace,
              metrics: rust.traces[shape].metrics,
              failures: rust.traces[shape].failures,
              hostEvent: rust.traces[shape].hostEvent,
            },
          ]),
        ),
        typescriptArtifacts: typescript.identities,
        consumers,
        audits: {
          generatedBindingsDrift: false,
          nativeRendererSemanticsEqual: true,
          rendererFixturesPayloadFree,
          documentOptionalEdgesAbsent: true,
          linearTreeEdgeAbsent: true,
          prototypeResolutionAbsent: true,
          peerRuntimeUnique: true,
          capabilitiesExact: true,
          productAuthorityRemainsConsumerOwned: true,
          productApplyRemainsConsumerOwned: true,
          checkpointContentRemainsConsumerOwned: true,
          publicPoodleOnly: true,
          registryPublicationAttempted: false,
          donorRepositoriesWritten: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_HISTORY_TREE_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
  await buildScratch.dispose();
}

async function packTypescriptArtifacts(): Promise<{
  identities: ReadonlyArray<ArtifactIdentity>;
  paths: ReadonlyMap<string, string>;
}> {
  const packages = [
    ["@inflatable-cookie/longhorn", "longhorn"],
    ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"],
    ["@inflatable-cookie/longhorn-tauri", "longhorn-tauri"],
  ] as const;
  const identities = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    await run(
      ["bun", "pm", "pack", "--destination", typescriptArtifactRoot, "--ignore-scripts", "--quiet"],
      join(repoRoot, "packages", directory),
    );
    const path = join(
      typescriptArtifactRoot,
      `${name.replace("@", "").replace("/", "-")}-${LONGHORN_VERSION}.tgz`,
    );
    await inspectNpmArtifact(name, path);
    paths.set(name, path);
    identities.push({ name, filename: basename(path), sha256: await digest(path) });
  }
  return { identities, paths };
}

async function inspectNpmArtifact(name: string, path: string): Promise<void> {
  const listing = await run(["tar", "-tzf", path], typescriptArtifactRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains workspace material`);
  }
  const manifest = readPackageManifest(
    await run(["tar", "-xOzf", path, "package/package.json"], typescriptArtifactRoot),
  );
  if (manifest.name !== name || manifest.version !== LONGHORN_VERSION) {
    throw new Error(`${name} packed identity mismatch`);
  }
  assertExactSet(
    `${name} dependencies`,
    Object.keys(manifest.dependencies ?? {}),
    name === "@inflatable-cookie/longhorn-history-tree" ? ["@inflatable-cookie/longhorn-core"] : [],
  );
  if (name === "@inflatable-cookie/longhorn-history-tree") {
    assertExactSet("history-tree exports", Object.keys(manifest.exports ?? {}), [
      ".",
      "./package.json",
      "./poodle",
      "./protocol",
      "./svelte",
      "./tauri",
    ]);
    for (const peer of ["svelte", "@inflatable-cookie/poodle-svelte"]) {
      if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) {
        throw new Error(`${peer} is not an optional history-tree peer`);
      }
    }
  }
  const extractRoot = join(
    typescriptArtifactRoot,
    `inspect-${name.replace("@", "").replace("/", "-")}`,
  );
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], typescriptArtifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  for (const marker of ["PulseMutation", "PulseHistoryMutation", "projectVersion"]) {
    if (source.includes(marker)) {
      throw new Error(`${name} contains product marker ${marker}`);
    }
  }
}

async function packAndRunRustArtifacts(): Promise<{
  identities: ReadonlyArray<ArtifactIdentity>;
  graphs: Record<Shape, ReadonlyArray<string>>;
  traces: Record<Shape, Json>;
  testCount: number;
}> {
  const artifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const crates = [
    "longhorn-core",
    "longhorn-history",
    "longhorn-history-tree",
    "longhorn-tauri-history-tree",
  ] as const;
  const identities = [];
  for (const name of crates) {
    // Checkout-level and non-building: `package --list` runs through the switch
    // point at the MSRV pin. The `test`/`run`/`tree` calls below lease the
    // staged workspace instead.
    const inventory = await run(
      rustCommand(["package", "-p", name, "--list", "--allow-dirty"], MSRV_TOOLCHAIN),
      repoRoot,
    );
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) {
      throw new Error(`${name} package inventory is incomplete`);
    }
    const tarArchive = join(artifactRoot, `${name}-${LONGHORN_VERSION}.private.tar`);
    const archive = `${tarArchive}.gz`;
    await run(["tar", "-cf", tarArchive, "-C", repoRoot, `crates/${name}`], repoRoot);
    await run(["gzip", "-n", tarArchive], repoRoot);
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({ name, filename: basename(archive), sha256: await digest(archive) });
  }
  await mkdir(join(workspace, "consumers"));
  await cp(join(proofRoot, "rust"), join(workspace, "consumers"), { recursive: true });
  await writeFile(join(workspace, "Cargo.toml"), rustWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));

  const source = [
    await readSourceTree(join(workspace, "crates", "longhorn-history-tree", "src")),
    await readSourceTree(join(workspace, "crates", "longhorn-tauri-history-tree", "src")),
  ].join("\n");
  for (const marker of ["PulseMutation", "PulseHistoryMutation", "project_version", "prototype"]) {
    if (source.toLowerCase().includes(marker.toLowerCase())) {
      throw new Error(`Rust artifacts contain product/prototype marker ${marker}`);
    }
  }

  // One exec lease over the staged workspace at the MSRV pin, with private final
  // outputs on build scratch: the tool inside calls plain Cargo. The real
  // `target/` directory keeps the lease adoption-free; see
  // `prepareStagedWorkspace`.
  await prepareStagedWorkspace(workspace);
  const staged = (args: ReadonlyArray<string>) =>
    rustExec(privateCargoCommand(buildScratch.root, args), {
      manifestPath: join(workspace, "Cargo.toml"),
      toolchain: MSRV_TOOLCHAIN,
    });
  const testOutput = await run(
    staged(["cargo", "test", "-p", "longhorn-history-tree", "--all-features", "--offline"]),
    workspace,
  );
  const testCount = [...testOutput.matchAll(/test result: ok\. (\d+) passed/g)].reduce(
    (sum, match) => sum + Number(match[1]),
    0,
  );
  // 51 since Card 191 named the origin: the default path still reports it
  // after a prune -- which is what protection guarantees and what a lying
  // origin row would depend on -- and a continuation run reports its anchor
  // instead. 53 since Card 213 added the envelope property suite. 57 since
  // Card 222 pinned the host-facing error messages by content. 59 since
  // AlreadyAtTarget added two protocol round-trips on the same crate. 64
  // since task 099 added policy-weight/migration-error, near-ceiling prune,
  // pinned-spine protected-lineage, and lazy branch-page projection tests.
  if (testCount !== 64) {
    throw new Error(`artifact tree test count mismatch: ${testCount}`);
  }

  const traces: Record<Shape, Json> = { document: {}, loophole: {} };
  const graphs: Record<Shape, ReadonlyArray<string>> = { document: [], loophole: [] };
  for (const shape of ["document", "loophole"] as const) {
    const packageName = `longhorn-${shape}-history-tree-artifact-proof`;
    traces[shape] = parseTrace(
      await run(staged(["cargo", "run", "-p", packageName, "--offline", "--quiet"]), workspace),
    );
    const tree = await run(
      staged(["cargo", "tree", "-p", packageName, "--offline", "--prefix", "none"]),
      workspace,
    );
    graphs[shape] = longhornPackages(tree);
    assertExactSet(`${shape} Rust graph`, graphs[shape], policies[shape].rust);
    assertMetrics(shape, traces[shape]);
  }
  return { identities, graphs, traces, testCount };
}

async function verifyTypescriptConsumer(
  shape: Shape,
  artifacts: ReadonlyMap<string, string>,
  nativeTrace: Json,
) {
  const policy = policies[shape];
  const source = join(proofRoot, "consumers", shape);
  const stage = join(temporaryRoot, `consumer-${shape}-${randomUUID()}`);
  await mkdir(join(stage, "consumers"), { recursive: true });
  await cp(source, join(stage, "consumers", shape), { recursive: true });
  await cp(join(proofRoot, "consumer-tsconfig.json"), join(stage, "consumer-tsconfig.json"));
  await writeFile(
    join(stage, "consumers", shape, "fixture.json"),
    `${JSON.stringify(nativeTrace, null, 2)}\n`,
  );
  const manifest = readPackageManifest(await readFile(join(source, "package.json"), "utf8"));
  manifest.dependencies = Object.fromEntries(
    Object.entries(manifest.dependencies ?? {}).map(([name, version]) => [
      name,
      artifacts.has(name)
        ? fileDependency(requireValue(artifacts.get(name), "artifacts.get(name)"))
        : version,
    ]),
  );
  // Only Longhorn's own packs are overridden onto paths; Poodle resolves from
  // the registry exactly as a real consumer resolves it.
  manifest.overrides = Object.fromEntries(
    [...artifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  await installProofStage(stage);
  await run(["bun", "x", "tsc", "-p", "consumer-tsconfig.json"], stage);
  const trace = parseTrace(await run(["bun", `consumers/${shape}/proof.ts`], stage));
  if (!equalJson(trace.publicTrace, nativeTrace.publicTrace)) {
    throw new Error(`${shape} native and renderer traces diverged`);
  }

  assertExactSet(
    `${shape} installed Longhorn packages`,
    (await installedScope(stage, "@inflatable-cookie"))
      .filter((name) => name === "longhorn" || name.startsWith("longhorn-"))
      .map((name) => `@inflatable-cookie/${name}`),
    policy.longhorn,
  );
  const artifactResolution = [];
  for (const name of policy.longhorn) {
    artifactResolution.push(await assertArtifactInstall(stage, name, LONGHORN_VERSION));
  }
  for (const name of policy.forbidden) {
    await assertPackageAbsent(stage, name);
  }
  if (shape === "loophole") {
    for (const pkg of POODLE_RELEASE.packages) {
      await assertArtifactInstall(stage, pkg.name, POODLE_RELEASE.version);
    }
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.56.8") {
      throw new Error("Loophole installed unexpected Svelte version");
    }
    await assertSingleSvelteRuntime(stage);
  } else if (
    (await installedScope(stage, "@inflatable-cookie")).some((name) => name.startsWith("poodle-"))
  ) {
    throw new Error("document consumer acquired Poodle");
  }
  const permissions =
    shape === "loophole"
      ? readPermissions(await readFile(join(stage, "consumers", shape, "capability.json"), "utf8"))
          .permissions
      : [];
  assertExactSet(`${shape} permissions`, permissions, policy.permissions);
  assertExactSet(`${shape} imports`, await longhornImports(stage), policy.imports);
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  if (
    lock.includes("workspace:") ||
    lock.includes("link:") ||
    lock.includes(resolve(repoRoot, "packages")) ||
    lock.includes("/Dev/projects/poodle/packages/")
  ) {
    throw new Error(`${shape} lockfile contains sibling source resolution`);
  }
  return {
    shape,
    longhornPackages: policy.longhorn,
    imports: policy.imports,
    permissions,
    forbiddenPackagesAbsent: policy.forbidden,
    artifactResolution,
    poodleVersion: shape === "loophole" ? POODLE_RELEASE.version : null,
    trace,
    cleanInstall: true,
    siblingSourceAliases: false,
  };
}

function assertMetrics(shape: Shape, trace: Json): void {
  const expected = policies[shape].metrics;
  const metrics = requireRecord(trace.metrics, "history-tree metrics");
  for (const [key, value] of [
    ["retainedNodes", expected.nodes],
    ["branchRefs", expected.branches],
    ["retainedPayloadBytes", expected.payload],
    ["lcaCheckoutSteps", expected.lca],
    ["numericArrayBaselineBytes", expected.baseline],
  ] as const) {
    if (metrics[key] !== value) {
      throw new Error(`${shape} ${key} mismatch: ${metrics[key]} != ${value}`);
    }
  }
  if (
    metrics.pathRecordsReturned !== 17 ||
    metrics.branchRecordsReturned !== Math.min(17, expected.branches)
  ) {
    throw new Error(`${shape} projection bound mismatch`);
  }
  const dense = metrics.denseEnvelopeBytes;
  if (typeof dense !== "number" || dense >= expected.baseline) {
    throw new Error(`${shape} dense envelope did not improve the Card 068 baseline`);
  }
  if (shape === "loophole" && dense * 2 >= expected.baseline) {
    throw new Error("Loophole dense envelope lost the Card 072 2x floor");
  }
  if (
    !Object.values(requireRecord(trace.failures, "history-tree failures")).every(
      (value) => value === true || value === 0,
    )
  ) {
    throw new Error(`${shape} failure matrix incomplete`);
  }
}

function rustWorkspaceManifest(): string {
  return `[workspace]
members = [
  "consumers/document",
  "consumers/loophole",
  "crates/longhorn-core",
  "crates/longhorn-history",
  "crates/longhorn-history-tree",
  "crates/longhorn-tauri-history-tree",
]
resolver = "2"

[workspace.package]
version = "${LONGHORN_VERSION}"
edition = "2024"
rust-version = "${MSRV}"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
base64 = "0.22.1"
longhorn-core = { path = "crates/longhorn-core", version = "${LONGHORN_VERSION}" }
longhorn-history = { path = "crates/longhorn-history", version = "${LONGHORN_VERSION}" }
longhorn-history-tree = { path = "crates/longhorn-history-tree", version = "${LONGHORN_VERSION}" }
longhorn-tauri-history-tree = { path = "crates/longhorn-tauri-history-tree", version = "${LONGHORN_VERSION}" }
${workspaceDependencies(["proptest", "serde", "serde_json", "tauri", "ts-rs"])}

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}

function containsKey(value: unknown, key: string): boolean {
  if (Array.isArray(value)) {
    return value.some((entry) => containsKey(entry, key));
  }
  if (value !== null && typeof value === "object") {
    return Object.entries(value).some(
      ([candidate, entry]) => candidate === key || containsKey(entry, key),
    );
  }
  return false;
}
function equalJson(left: unknown, right: unknown): boolean {
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}
// oxlint-disable-next-line quality/no-unknown-returns -- Canonicalize arbitrary JSON for comparison without assuming its shape.
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(canonical);
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, entry]) => [key, canonical(entry)]),
    );
  }
  return value;
}
async function readSourceTree(root: string): Promise<string> {
  const files = (await readdir(root, { recursive: true }))
    .filter((path) => /\.(rs|ts|svelte)$/.test(path))
    .sort();
  return (await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))).join("\n");
}
function longhornPackages(tree: string): ReadonlyArray<string> {
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
async function installedScope(stage: string, scope: string): Promise<ReadonlyArray<string>> {
  try {
    return (await readdir(join(stage, "node_modules", scope))).sort();
  } catch (error) {
    if (errorCode(error) === "ENOENT") {
      return [];
    }
    throw error;
  }
}
async function assertArtifactInstall(stage: string, name: string, expected: string) {
  const installed = await installedPackage(stage, name);
  for (const root of ["/Dev/projects/longhorn/packages/", "/Dev/projects/poodle/packages/"]) {
    if (installed.realPath.includes(root)) {
      throw new Error(`${name} resolved to sibling source: ${installed.realPath}`);
    }
  }
  if (installed.manifest.version !== expected) {
    throw new Error(`${name} installed ${installed.manifest.version}, expected ${expected}`);
  }
  return { name, version: installed.manifest.version };
}
async function installedPackage(stage: string, name: string) {
  const path = join(stage, "node_modules", ...name.split("/"));
  const manifest = readPackageIdentity(await readFile(join(path, "package.json"), "utf8"));
  if (manifest.name !== name) {
    throw new Error(`installed package identity mismatch for ${name}`);
  }
  return { realPath: await realpath(path), manifest };
}
async function assertPackageAbsent(stage: string, name: string): Promise<void> {
  try {
    await lstat(join(stage, "node_modules", ...name.split("/")));
  } catch (error) {
    if (errorCode(error) === "ENOENT") {
      return;
    }
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the install graph`);
}
async function assertSingleSvelteRuntime(stage: string): Promise<void> {
  const manifests = (await readdir(join(stage, "node_modules"), { recursive: true })).filter(
    (path) => path === "svelte/package.json" || path.endsWith("/node_modules/svelte/package.json"),
  );
  if (manifests.length !== 1) {
    throw new Error(`expected one Svelte runtime, found ${manifests.length}`);
  }
}
async function longhornImports(stage: string): Promise<ReadonlyArray<string>> {
  const files = (await readdir(stage, { recursive: true })).filter(
    (path) => /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules/"),
  );
  const imports = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(stage, path), "utf8");
    for (const match of source.matchAll(
      /from\s+["'](@inflatable-cookie\/longhorn(?:[/-][^"']*)?)["']/g,
    )) {
      imports.add(requireValue(match[1], "match[1]"));
    }
  }
  return [...imports].sort();
}
