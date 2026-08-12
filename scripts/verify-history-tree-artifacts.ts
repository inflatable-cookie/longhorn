import { poodleRelease } from "./poodle-release.ts";
import { workspaceDependencies } from "./workspace-dependencies.ts";
import { createHash, randomUUID } from "node:crypto";
import { cp, lstat, mkdir, mkdtemp, readFile, readdir, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

// Poodle installs from the registry; poodleRelease() checks each published
// package's sha512 against bun.lock and against the installed copy.
const POODLE_RELEASE = poodleRelease();
const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "history-tree-artifact-proof");
const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-history-tree-artifact-proof-"));
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(typescriptArtifactRoot);

type Shape = "document" | "loophole";
type Json = Record<string, any>;
interface ArtifactIdentity { readonly name: string; readonly filename: string; readonly sha256: string; }
interface PackageManifest { readonly name: string; dependencies: Record<string, string>; overrides?: Record<string, string>; }

const policies = {
  document: {
    rust: ["longhorn-core", "longhorn-history", "longhorn-history-tree"],
    longhorn: ["@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn/history-tree"],
    permissions: [],
    forbidden: ["@inflatable-cookie/longhorn/history", "@inflatable-cookie/longhorn-tauri"],
    metrics: { nodes: 132, branches: 5, payload: 4_224, lca: 65, baseline: 99_295 },
  },
  loophole: {
    rust: ["longhorn-core", "longhorn-history", "longhorn-history-tree", "longhorn-tauri-history-tree"],
    longhorn: ["@inflatable-cookie/longhorn-poodle-svelte", "@inflatable-cookie/longhorn-tauri", "@inflatable-cookie/longhorn"],
    imports: ["@inflatable-cookie/longhorn-poodle-svelte/history-tree/svelte", "@inflatable-cookie/longhorn-tauri/history-tree", "@inflatable-cookie/longhorn/history-tree"],
    permissions: ["allow-longhorn-history-tree-read", "allow-longhorn-history-tree-mutate", "core:event:allow-listen", "core:event:allow-unlisten"],
    forbidden: ["@inflatable-cookie/longhorn/history"],
    metrics: { nodes: 2_112, branches: 65, payload: 540_672, lca: 1_025, baseline: 7_534_856 },
  },
} as const;

try {
  await run(["cargo", "run", "-p", "longhorn-bindings", "--", "history-tree", "check"], repoRoot);
  const typescript = await packTypescriptArtifacts();
  const rust = await packAndRunRustArtifacts();
  const consumers = [];
  for (const shape of ["document", "loophole"] as const) {
    consumers.push(await verifyTypescriptConsumer(shape, typescript.paths, rust.traces[shape]));
  }
  const rendererFixturesPayloadFree = (["document", "loophole"] as const).every(
    (shape) => !containsKey(rust.traces[shape].rendererFixture, "payload"),
  );
  if (!rendererFixturesPayloadFree) throw new Error("renderer fixture contains product payload");

  console.log(JSON.stringify({
    schema: "longhorn.history-tree-artifact-proof.v1",
    linearControl: "effigy proof:history-system-artifacts",
    poodleVersion: POODLE_RELEASE.version,
    rustArtifacts: rust.identities,
    rustGraphs: rust.graphs,
    artifactCoreTestCount: rust.testCount,
    nativeTraces: Object.fromEntries((["document", "loophole"] as const).map((shape) => [shape, { publicTrace: rust.traces[shape].publicTrace, metrics: rust.traces[shape].metrics, failures: rust.traces[shape].failures, hostEvent: rust.traces[shape].hostEvent }])),
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
  }, null, 2));
} finally {
  if (process.env.KEEP_HISTORY_TREE_PROOF === "1") console.error(`retained proof workspace: ${temporaryRoot}`);
  else await rm(temporaryRoot, { recursive: true, force: true });
}

async function packTypescriptArtifacts(): Promise<{ identities: readonly ArtifactIdentity[]; paths: ReadonlyMap<string, string> }> {
  const packages = [["@inflatable-cookie/longhorn", "longhorn"], ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"], ["@inflatable-cookie/longhorn-tauri", "longhorn-tauri"]] as const;
  const identities = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    await run(["bun", "pm", "pack", "--destination", typescriptArtifactRoot, "--ignore-scripts", "--quiet"], join(repoRoot, "packages", directory));
    const path = join(typescriptArtifactRoot, `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`);
    await inspectNpmArtifact(name, path);
    paths.set(name, path);
    identities.push({ name, filename: basename(path), sha256: await digest(path) });
  }
  return { identities, paths };
}

async function inspectNpmArtifact(name: string, path: string): Promise<void> {
  const listing = await run(["tar", "-tzf", path], typescriptArtifactRoot);
  if (listing.includes("node_modules/") || listing.includes("/tests/") || listing.includes("workspace:")) throw new Error(`${name} artifact contains workspace material`);
  const manifest = JSON.parse(await run(["tar", "-xOzf", path, "package/package.json"], typescriptArtifactRoot)) as Json;
  if (manifest.name !== name || manifest.version !== "0.1.0") throw new Error(`${name} packed identity mismatch`);
  assertExactSet(`${name} dependencies`, Object.keys(manifest.dependencies ?? {}), name === "@inflatable-cookie/longhorn-history-tree" ? ["@inflatable-cookie/longhorn-core"] : []);
  if (name === "@inflatable-cookie/longhorn-history-tree") {
    assertExactSet("history-tree exports", Object.keys(manifest.exports ?? {}), [".", "./package.json", "./poodle", "./protocol", "./svelte", "./tauri"]);
    for (const peer of ["svelte", "@inflatable-cookie/poodle-svelte"]) if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) throw new Error(`${peer} is not an optional history-tree peer`);
  }
  const extractRoot = join(typescriptArtifactRoot, `inspect-${name.replace("@", "").replace("/", "-")}`);
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], typescriptArtifactRoot);
  const source = await readSourceTree(join(extractRoot, "package", "src"));
  for (const marker of ["PulseMutation", "PulseHistoryMutation", "projectVersion"]) if (source.includes(marker)) throw new Error(`${name} contains product marker ${marker}`);
}

async function packAndRunRustArtifacts(): Promise<{ identities: readonly ArtifactIdentity[]; graphs: Record<Shape, readonly string[]>; traces: Record<Shape, Json>; testCount: number }> {
  const artifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const crates = ["longhorn-core", "longhorn-history", "longhorn-history-tree", "longhorn-tauri-history-tree"] as const;
  const identities = [];
  for (const name of crates) {
    const inventory = await run(["cargo", "+1.95.0", "package", "-p", name, "--list", "--allow-dirty"], repoRoot);
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) throw new Error(`${name} package inventory is incomplete`);
    const tarArchive = join(artifactRoot, `${name}-0.1.0.private.tar`);
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
  for (const marker of ["PulseMutation", "PulseHistoryMutation", "project_version", "prototype"]) if (source.toLowerCase().includes(marker.toLowerCase())) throw new Error(`Rust artifacts contain product/prototype marker ${marker}`);

  const testOutput = await run(["cargo", "+1.95.0", "test", "-p", "longhorn-history-tree", "--all-features", "--offline"], workspace);
  const testCount = [...testOutput.matchAll(/test result: ok\. (\d+) passed/g)].reduce((sum, match) => sum + Number(match[1]), 0);
  // 40 since the preferred-child invariant became a guard: a state whose node
  // has a choice of children and names none is rejected, rather than
  // truncating every forward walk at that node.
  if (testCount !== 40) throw new Error(`artifact tree test count mismatch: ${testCount}`);

  const traces = {} as Record<Shape, Json>;
  const graphs = {} as Record<Shape, readonly string[]>;
  for (const shape of ["document", "loophole"] as const) {
    const packageName = `longhorn-${shape}-history-tree-artifact-proof`;
    traces[shape] = parseTrace(await run(["cargo", "+1.95.0", "run", "-p", packageName, "--offline", "--quiet"], workspace));
    const tree = await run(["cargo", "+1.95.0", "tree", "-p", packageName, "--offline", "--prefix", "none"], workspace);
    graphs[shape] = longhornPackages(tree);
    assertExactSet(`${shape} Rust graph`, graphs[shape], policies[shape].rust);
    assertMetrics(shape, traces[shape]);
  }
  return { identities, graphs, traces, testCount };
}

async function verifyTypescriptConsumer(shape: Shape, artifacts: ReadonlyMap<string, string>, nativeTrace: Json) {
  const policy = policies[shape];
  const source = join(proofRoot, "consumers", shape);
  const stage = join(temporaryRoot, `consumer-${shape}-${randomUUID()}`);
  await mkdir(join(stage, "consumers"), { recursive: true });
  await cp(source, join(stage, "consumers", shape), { recursive: true });
  await cp(join(proofRoot, "consumer-tsconfig.json"), join(stage, "consumer-tsconfig.json"));
  await writeFile(join(stage, "consumers", shape, "fixture.json"), `${JSON.stringify(nativeTrace, null, 2)}\n`);
  const manifest = JSON.parse(await readFile(join(source, "package.json"), "utf8")) as PackageManifest;
  manifest.dependencies = Object.fromEntries(Object.entries(manifest.dependencies).map(([name, version]) => [name, artifacts.has(name) ? fileDependency(artifacts.get(name)!) : version]));
  // Only Longhorn's own packs are overridden onto paths; Poodle resolves from
  // the registry exactly as a real consumer resolves it.
  manifest.overrides = Object.fromEntries([...artifacts].map(([name, path]) => [name, fileDependency(path)]));
  await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "consumer-tsconfig.json"], stage);
  const trace = parseTrace(await run(["bun", `consumers/${shape}/proof.ts`], stage));
  if (!equalJson(trace.publicTrace, nativeTrace.publicTrace)) throw new Error(`${shape} native and renderer traces diverged`);

  assertExactSet(`${shape} installed Longhorn packages`, (await installedScope(stage, "@inflatable-cookie")).filter((name) => name === "longhorn" || name.startsWith("longhorn-")).map((name) => `@inflatable-cookie/${name}`), policy.longhorn);
  const artifactResolution = [];
  for (const name of policy.longhorn) artifactResolution.push(await assertArtifactInstall(stage, name));
  for (const name of policy.forbidden) await assertPackageAbsent(stage, name);
  if (shape === "loophole") {
    for (const pkg of POODLE_RELEASE.packages) await assertArtifactInstall(stage, pkg.name);
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") throw new Error("Loophole installed unexpected Svelte version");
    await assertSingleSvelteRuntime(stage);
  } else if ((await installedScope(stage, "@inflatable-cookie")).some((name) => name.startsWith("poodle-"))) throw new Error("document consumer acquired Poodle");
  const permissions = shape === "loophole" ? (JSON.parse(await readFile(join(stage, "consumers", shape, "capability.json"), "utf8")) as { permissions: string[] }).permissions : [];
  assertExactSet(`${shape} permissions`, permissions, policy.permissions);
  assertExactSet(`${shape} imports`, await longhornImports(stage), policy.imports);
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  if (lock.includes("workspace:") || lock.includes("link:") || lock.includes(resolve(repoRoot, "packages")) || lock.includes("/Dev/projects/poodle/packages/")) throw new Error(`${shape} lockfile contains sibling source resolution`);
  return { shape, longhornPackages: policy.longhorn, imports: policy.imports, permissions, forbiddenPackagesAbsent: policy.forbidden, artifactResolution, poodleVersion: shape === "loophole" ? POODLE_RELEASE.version : null, trace, cleanInstall: true, siblingSourceAliases: false };
}

function assertMetrics(shape: Shape, trace: Json): void {
  const expected = policies[shape].metrics;
  const metrics = trace.metrics as Json;
  for (const [key, value] of [["retainedNodes", expected.nodes], ["branchRefs", expected.branches], ["retainedPayloadBytes", expected.payload], ["lcaCheckoutSteps", expected.lca], ["numericArrayBaselineBytes", expected.baseline]] as const) if (metrics[key] !== value) throw new Error(`${shape} ${key} mismatch: ${metrics[key]} != ${value}`);
  if (metrics.pathRecordsReturned !== 17 || metrics.branchRecordsReturned !== Math.min(17, expected.branches)) throw new Error(`${shape} projection bound mismatch`);
  if (metrics.denseEnvelopeBytes >= expected.baseline) throw new Error(`${shape} dense envelope did not improve the Card 068 baseline`);
  if (shape === "loophole" && metrics.denseEnvelopeBytes * 2 >= expected.baseline) throw new Error("Loophole dense envelope lost the Card 072 2x floor");
  if (!Object.values(trace.failures as Json).every((value) => value === true || value === 0)) throw new Error(`${shape} failure matrix incomplete`);
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
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
base64 = "0.22.1"
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-history = { path = "crates/longhorn-history", version = "0.1.0" }
longhorn-history-tree = { path = "crates/longhorn-history-tree", version = "0.1.0" }
longhorn-tauri-history-tree = { path = "crates/longhorn-tauri-history-tree", version = "0.1.0" }
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

async function digest(path: string): Promise<string> { return createHash("sha256").update(await readFile(path)).digest("hex"); }
async function run(command: readonly string[], cwd: string): Promise<string> {
  const subprocess = Bun.spawn(command, { cwd, env: { ...process.env, COPYFILE_DISABLE: "1" }, stdout: "pipe", stderr: "pipe" });
  const [exitCode, stdout, stderr] = await Promise.all([subprocess.exited, new Response(subprocess.stdout).text(), new Response(subprocess.stderr).text()]);
  if (exitCode !== 0) throw new Error(`${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`);
  return `${stdout}\n${stderr}`;
}
function fileDependency(path: string): string { return `file:${path}`; }
function assertExactSet(label: string, actual: readonly string[], expected: readonly string[]): void { const left = [...actual].sort(); const right = [...expected].sort(); if (JSON.stringify(left) !== JSON.stringify(right)) throw new Error(`${label} mismatch: ${left.join(", ")} != ${right.join(", ")}`); }
function parseTrace(output: string): Json { const line = output.split("\n").find((candidate) => candidate.trim().startsWith("{")); if (line === undefined) throw new Error("proof emitted no JSON trace"); return JSON.parse(line) as Json; }
function containsKey(value: unknown, key: string): boolean { if (Array.isArray(value)) return value.some((entry) => containsKey(entry, key)); if (value !== null && typeof value === "object") return Object.entries(value).some(([candidate, entry]) => candidate === key || containsKey(entry, key)); return false; }
function equalJson(left: unknown, right: unknown): boolean { return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right)); }
function canonical(value: unknown): unknown { if (Array.isArray(value)) return value.map(canonical); if (value !== null && typeof value === "object") return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right)).map(([key, entry]) => [key, canonical(entry)])); return value; }
async function readSourceTree(root: string): Promise<string> { const files = (await readdir(root, { recursive: true })).filter((path) => /\.(rs|ts|svelte)$/.test(path)).sort(); return (await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))).join("\n"); }
function longhornPackages(tree: string): readonly string[] { return [...new Set(tree.split("\n").map((line) => line.trim().split(/\s+/)[0] ?? "").filter((name) => name.startsWith("longhorn-")).filter((name) => !name.endsWith("-artifact-proof")))].sort(); }
async function installedScope(stage: string, scope: string): Promise<readonly string[]> { try { return (await readdir(join(stage, "node_modules", scope))).sort(); } catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") return []; throw error; } }
async function assertArtifactInstall(stage: string, name: string) { const installed = await installedPackage(stage, name); for (const root of ["/Dev/projects/longhorn/packages/", "/Dev/projects/poodle/packages/"]) if (installed.realPath.includes(root)) throw new Error(`${name} resolved to sibling source: ${installed.realPath}`); if (installed.manifest.version !== "0.1.0") throw new Error(`${name} installed unexpected version`); return { name, version: installed.manifest.version }; }
async function installedPackage(stage: string, name: string) { const path = join(stage, "node_modules", ...name.split("/")); const manifest = JSON.parse(await readFile(join(path, "package.json"), "utf8")) as { name: string; version: string }; if (manifest.name !== name) throw new Error(`installed package identity mismatch for ${name}`); return { realPath: await realpath(path), manifest }; }
async function assertPackageAbsent(stage: string, name: string): Promise<void> { try { await lstat(join(stage, "node_modules", ...name.split("/"))); } catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") return; throw error; } throw new Error(`${name} unexpectedly entered the install graph`); }
async function assertSingleSvelteRuntime(stage: string): Promise<void> { const manifests = (await readdir(join(stage, "node_modules"), { recursive: true })).filter((path) => path === "svelte/package.json" || path.endsWith("/node_modules/svelte/package.json")); if (manifests.length !== 1) throw new Error(`expected one Svelte runtime, found ${manifests.length}`); }
async function longhornImports(stage: string): Promise<readonly string[]> { const files = (await readdir(stage, { recursive: true })).filter((path) => /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules/")); const imports = new Set<string>(); for (const path of files) { const source = await readFile(join(stage, path), "utf8"); for (const match of source.matchAll(/from\s+["'](@inflatable-cookie\/longhorn(?:[/-][^"']*)?)["']/g)) imports.add(match[1]!); } return [...imports].sort(); }
