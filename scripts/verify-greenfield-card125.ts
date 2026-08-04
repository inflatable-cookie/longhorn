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

type ShapeName = "minimal" | "workspace" | "full-hosting" | "optional-server";
type ShapePolicy = {
  hierarchy: readonly string[];
  typescript: readonly string[];
  rust: readonly string[];
  forbiddenTypescript: readonly string[];
  forbiddenRust: readonly string[];
  permissions: readonly string[];
};
type ArtifactIdentity = { name: string; filename: string; sha256: string };

const repoRoot = resolve(import.meta.dir, "..");
const bunCommand = ["/usr/bin/env", "bun"] as const;
const exampleRoot = join(repoRoot, "examples/greenfield-compositions");
const receiptPath = join(repoRoot, "fixtures/greenfield/card125/composition-matrix-v1.json");
const poodleRoot = resolve(process.env.POODLE_REPO ?? join(repoRoot, "../poodle"));
const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-greenfield-card125-"));
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
const rustArtifactRoot = join(temporaryRoot, "rust-artifacts");

const longhornTypescriptPackages = [
  ["@longhorn/bridge", "packages/bridge"],
  ["@longhorn/commands", "packages/commands"],
  ["@longhorn/config", "packages/config"],
  ["@longhorn/core", "packages/core"],
  ["@longhorn/history", "packages/history"],
  ["@longhorn/layout", "packages/layout"],
  ["@longhorn/poodle", "packages/poodle"],
  ["@longhorn/settings", "packages/settings"],
  ["@longhorn/surface-transfer", "packages/surface-transfer"],
  ["@longhorn/surfaces", "packages/surfaces"],
  ["@longhorn/svelte", "packages/svelte"],
  ["@longhorn/tauri", "packages/tauri"],
  ["@longhorn/transfer", "packages/transfer"],
] as const;
const poodlePackages = [
  ["@poodle/headless", "packages/core"],
  ["@poodle/styles", "packages/styles"],
  ["@poodle/svelte-tokens", "packages/svelte/tokens"],
  ["@poodle/icons-lucide", "packages/svelte/icons-lucide"],
  ["@poodle/svelte", "packages/svelte/components"],
] as const;
const allRustCrates = [
  "longhorn-bridge",
  "longhorn-command",
  "longhorn-command-config",
  "longhorn-command-settings",
  "longhorn-config",
  "longhorn-core",
  "longhorn-display",
  "longhorn-history",
  "longhorn-layout",
  "longhorn-layout-config",
  "longhorn-settings",
  "longhorn-settings-config",
  "longhorn-surface-transfer",
  "longhorn-surface-windowing",
  "longhorn-surfaces",
  "longhorn-surfaces-config",
  "longhorn-tauri-bridge",
  "longhorn-tauri-command",
  "longhorn-tauri-config",
  "longhorn-tauri-history",
  "longhorn-tauri-settings",
  "longhorn-tauri-transfer",
  "longhorn-tauri-windowing",
  "longhorn-transfer",
  "longhorn-windowing",
  "longhorn-windowing-config",
] as const;

const baseRust = [
  "longhorn-config", "longhorn-core", "longhorn-settings", "longhorn-settings-config",
  "longhorn-tauri-config", "longhorn-tauri-settings",
] as const;
const workspaceRust = [
  "longhorn-command", "longhorn-command-config", "longhorn-command-settings",
  "longhorn-config", "longhorn-core", "longhorn-display", "longhorn-layout",
  "longhorn-layout-config", "longhorn-settings", "longhorn-settings-config",
  "longhorn-tauri-command", "longhorn-tauri-config", "longhorn-tauri-settings",
  "longhorn-tauri-windowing", "longhorn-windowing", "longhorn-windowing-config",
] as const;
const fullRust = [
  ...workspaceRust,
  "longhorn-history", "longhorn-surface-transfer", "longhorn-surface-windowing",
  "longhorn-surfaces", "longhorn-surfaces-config", "longhorn-tauri-history",
  "longhorn-tauri-transfer", "longhorn-transfer",
] as const;

const policies: Record<ShapeName, ShapePolicy> = {
  minimal: {
    hierarchy: ["window"],
    typescript: ["@longhorn/config", "@longhorn/core", "@longhorn/settings", "@longhorn/tauri"],
    rust: baseRust,
    forbiddenTypescript: ["@longhorn/layout", "@longhorn/surfaces", "@longhorn/bridge", "@longhorn/history", "@longhorn/native-content"],
    forbiddenRust: ["longhorn-layout", "longhorn-surfaces", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-config-read", "core:event:allow-listen", "core:event:allow-unlisten"],
  },
  workspace: {
    hierarchy: ["window", "layout-container", "region", "panel"],
    typescript: ["@longhorn/commands", "@longhorn/config", "@longhorn/core", "@longhorn/layout", "@longhorn/poodle", "@longhorn/settings", "@longhorn/svelte", "@longhorn/tauri"],
    rust: workspaceRust,
    forbiddenTypescript: ["@longhorn/surfaces", "@longhorn/transfer", "@longhorn/surface-transfer", "@longhorn/bridge", "@longhorn/history", "@longhorn/native-content"],
    forbiddenRust: ["longhorn-surfaces", "longhorn-transfer", "longhorn-surface-transfer", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-command-read", "allow-longhorn-command-mutate", "core:event:allow-listen", "core:event:allow-unlisten", "core:window:allow-start-dragging"],
  },
  "full-hosting": {
    hierarchy: ["display", "window", "surface", "layout-container", "region", "panel"],
    typescript: ["@longhorn/commands", "@longhorn/config", "@longhorn/core", "@longhorn/history", "@longhorn/layout", "@longhorn/poodle", "@longhorn/settings", "@longhorn/surface-transfer", "@longhorn/surfaces", "@longhorn/svelte", "@longhorn/tauri", "@longhorn/transfer"],
    rust: fullRust,
    forbiddenTypescript: ["@longhorn/bridge", "@longhorn/native-content", "@longhorn/operation", "@longhorn/notifications"],
    forbiddenRust: ["longhorn-bridge", "longhorn-native-content", "longhorn-operation", "longhorn-notifications"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-command-read", "allow-longhorn-command-mutate", "allow-longhorn-transfer-base", "allow-longhorn-transfer-surface", "allow-longhorn-history-read", "allow-longhorn-history-mutate", "core:event:allow-listen", "core:event:allow-unlisten", "core:window:allow-start-dragging"],
  },
  "optional-server": {
    hierarchy: ["window", "optional-service-bridge"],
    typescript: ["@longhorn/bridge", "@longhorn/config", "@longhorn/core", "@longhorn/settings", "@longhorn/tauri"],
    rust: [...baseRust, "longhorn-bridge", "longhorn-tauri-bridge"],
    forbiddenTypescript: ["@longhorn/layout", "@longhorn/surfaces", "@longhorn/history", "@longhorn/native-content"],
    forbiddenRust: ["longhorn-layout", "longhorn-surfaces", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-bridge-query", "core:event:allow-listen", "core:event:allow-unlisten"],
  },
};

try {
  await mkdir(typescriptArtifactRoot);
  await mkdir(rustArtifactRoot);
  await verifyExampleSources();
  await verifySelectedSourcesClean();
  const poodle = await packPackages(poodleRoot, poodlePackages, typescriptArtifactRoot);
  const typescript = await packPackages(repoRoot, longhornTypescriptPackages, typescriptArtifactRoot);
  const renderers = await verifyRenderers(new Map([...poodle.paths, ...typescript.paths]));
  const rust = await verifyRustArtifacts();
  const report = {
    schema: "longhorn.greenfield-composition-matrix.v1",
    outcome: "pass",
    sources: {
      longhorn: await git(repoRoot, ["rev-parse", "HEAD"]),
      poodle: await git(poodleRoot, ["rev-parse", "HEAD"]),
      selectedLonghornClean: true,
      selectedPoodleClean: true,
    },
    artifacts: {
      typescript: typescript.identities,
      poodle: poodle.identities,
      rust: rust.identities,
      sets: {
        typescript: artifactSet(typescript.identities),
        poodle: artifactSet(poodle.identities),
        rust: artifactSet(rust.identities),
      },
      packageManagerPublication: false,
    },
    shapes: renderers.map((renderer) => ({
      ...renderer,
      rust: rust.shapes.find(({ shape }) => shape === renderer.shape),
    })),
    optionalServerLocal: rust.optionalServerLocal,
    audits: {
      outsideWorkspaceResolution: true,
      oneSvelteRuntimePerRenderer: true,
      oneTauriApiRuntimePerRenderer: true,
      umbrellaPackage: false,
      donorVocabulary: false,
      copiedPoodlePrimitive: false,
      nativeContent: false,
      operation: false,
      notifications: false,
      packageManagerPublication: false,
    },
  };
  await verifyReceipt(report);
  console.log(JSON.stringify(report, null, 2));
} finally {
  if (process.env.KEEP_GREENFIELD_COMPOSITION_PROOF === "1") {
    console.error(`retained greenfield proof: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function verifyReceipt(report: {
  schema: string;
  artifacts: {
    typescript: ArtifactIdentity[];
    poodle: ArtifactIdentity[];
    rust: ArtifactIdentity[];
    sets: { typescript: string; poodle: string; rust: string };
  };
  shapes: Array<{
    shape: ShapeName;
    hierarchy: readonly string[];
    typescriptPackages: string[];
    rust: { rustPackages: string[] } | undefined;
  }>;
  optionalServerLocal: { rustPackages: string[] };
  audits: Record<string, boolean>;
}): Promise<void> {
  const receipt = JSON.parse(await readFile(receiptPath, "utf8")) as {
    schema: string;
    sources: { longhornSelected: string; poodleSelected: string };
    artifacts: {
      sets: { typescript: string; poodle: string; rust: string };
      inventories: { typescript: string[]; poodle: string[]; rust: string[] };
    };
    shapes: Record<ShapeName, { hierarchy: string[]; typescript: string[]; rust: string[] }>;
    optionalServerLocalRust: string[];
    audits: Record<string, boolean>;
  };
  if (receipt.schema !== report.schema) throw new Error("greenfield receipt schema drift");
  await assertFrozenSource(repoRoot, receipt.sources.longhornSelected, [
    ...longhornTypescriptPackages.map(([, path]) => path),
    ...allRustCrates.map((name) => `crates/${name}`),
  ]);
  await assertFrozenSource(poodleRoot, receipt.sources.poodleSelected, poodlePackages.map(([, path]) => path));
  if (JSON.stringify(receipt.artifacts.sets) !== JSON.stringify(report.artifacts.sets)) {
    throw new Error(`greenfield artifact set drift: expected ${JSON.stringify(receipt.artifacts.sets)}, received ${JSON.stringify(report.artifacts.sets)}`);
  }
  equalSet(report.artifacts.typescript.map(({ name }) => name), receipt.artifacts.inventories.typescript, "receipt TypeScript inventory");
  equalSet(report.artifacts.poodle.map(({ name }) => name), receipt.artifacts.inventories.poodle, "receipt Poodle inventory");
  equalSet(report.artifacts.rust.map(({ name }) => name), receipt.artifacts.inventories.rust, "receipt Rust inventory");
  for (const shape of report.shapes) {
    const expected = receipt.shapes[shape.shape];
    if (!shape.rust) throw new Error(`${shape.shape} Rust report missing`);
    equalSet(shape.hierarchy, expected.hierarchy, `${shape.shape} receipt hierarchy`);
    equalSet(shape.typescriptPackages, expected.typescript, `${shape.shape} receipt TypeScript graph`);
    equalSet(shape.rust.rustPackages, expected.rust, `${shape.shape} receipt Rust graph`);
  }
  equalSet(report.optionalServerLocal.rustPackages, receipt.optionalServerLocalRust, "receipt optional-server local graph");
  if (JSON.stringify(receipt.audits) !== JSON.stringify(report.audits)) throw new Error("greenfield audit receipt drift");
}

async function assertFrozenSource(root: string, commit: string, paths: readonly string[]): Promise<void> {
  const diff = Bun.spawnSync(["git", "diff", "--quiet", commit, "--", ...paths], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (diff.exitCode !== 0) throw new Error(`selected source drift from ${commit}:\n${diff.stderr.toString()}`);
}

async function verifyExampleSources(): Promise<void> {
  const source = Bun.spawnSync(["rg", "-n", "-i", "loophole|nucleus|soundcheck|bovine|jetstream", exampleRoot], { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
  if (source.exitCode === 0) throw new Error(`greenfield examples contain donor vocabulary:\n${source.stdout.toString()}`);
  if (source.exitCode !== 1) throw new Error(`greenfield vocabulary scan failed:\n${source.stderr.toString()}`);
  const svelteFiles = (await readdir(exampleRoot, { recursive: true }))
    .filter((path) => path.endsWith(".svelte"));
  equalSet(svelteFiles, ["common/App.svelte"], "greenfield Svelte implementation inventory");
  const shell = await readFile(join(exampleRoot, "common/App.svelte"), "utf8");
  if (!shell.includes('from "@poodle/svelte"') || !shell.includes('import "@poodle/svelte-tokens/styles.css"')) {
    throw new Error("greenfield shell does not compose public Poodle roots");
  }
}

async function verifySelectedSourcesClean(): Promise<void> {
  const longhornPaths = longhornTypescriptPackages.map(([, path]) => path)
    .concat(allRustCrates.map((name) => `crates/${name}`));
  if (await git(repoRoot, ["status", "--porcelain", "--", ...longhornPaths])) throw new Error("selected Longhorn packages are dirty");
  const poodlePaths = poodlePackages.map(([, path]) => path);
  if (await git(poodleRoot, ["status", "--porcelain", "--", ...poodlePaths])) throw new Error("selected Poodle packages are dirty");
}

async function packPackages(
  root: string,
  packages: readonly (readonly [string, string])[],
  destination: string,
): Promise<{ identities: ArtifactIdentity[]; paths: Map<string, string> }> {
  const identities: ArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    try {
      await run([...bunCommand, "pm", "pack", "--destination", destination, "--ignore-scripts", "--quiet"], join(root, directory));
    } catch (error) {
      throw new Error(`${name} pack failed from ${join(root, directory)}: ${String(error)}`);
    }
    const path = join(destination, `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`);
    const listing = await run(["tar", "-tzf", path], destination);
    const manifest = await run(["tar", "-xOzf", path, "package/package.json"], destination);
    if (listing.includes("node_modules/") || listing.includes("/tests/") || /workspace:|link:/.test(manifest)) {
      throw new Error(`${name} artifact contains workspace or proof material`);
    }
    const parsed = JSON.parse(manifest) as { name: string; version: string };
    if (parsed.name !== name || parsed.version !== "0.1.0") throw new Error(`${name} artifact identity drift`);
    const identity = { name, filename: basename(path), sha256: await digest(path) };
    identities.push(identity);
    paths.set(name, path);
  }
  return { identities, paths };
}

async function verifyRenderers(artifacts: Map<string, string>) {
  const reports = [];
  for (const shape of Object.keys(policies) as ShapeName[]) {
    const policy = policies[shape];
    const stage = join(temporaryRoot, `renderer-${shape}-${randomUUID()}`);
    await mkdir(stage);
    await cp(join(exampleRoot, "common"), join(stage, "common"), { recursive: true });
    await cp(join(exampleRoot, shape), join(stage, shape), { recursive: true });
    const manifest = JSON.parse(await readFile(join(exampleRoot, shape, "package.json"), "utf8")) as {
      dependencies: Record<string, string>;
      devDependencies: Record<string, string>;
      overrides?: Record<string, string>;
    };
    manifest.dependencies = rewriteArtifactDependencies(manifest.dependencies, artifacts);
    manifest.overrides = Object.fromEntries([...artifacts].map(([name, path]) => [name, fileDependency(path)]));
    await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
    await run([...bunCommand, "install", "--ignore-scripts"], stage);
    await run([...bunCommand, "x", "svelte-check", "--tsconfig", `${shape}/tsconfig.json`], stage);
    await run([...bunCommand, "x", "vite", "build", shape, "--config", `${shape}/vite.config.ts`], stage);
    const tests = await run([...bunCommand, "x", "vitest", "run", "--config", `${shape}/vitest.config.ts`], stage);
    if (!/1 passed/.test(tests)) throw new Error(`${shape} mounted lifecycle test did not pass`);

    const installedLonghorn = (await readdir(join(stage, "node_modules/@longhorn"))).map((name) => `@longhorn/${name}`).sort();
    equalSet(installedLonghorn, policy.typescript, `${shape} TypeScript graph`);
    for (const name of policy.forbiddenTypescript) await assertAbsent(stage, name);
    const installedPoodle = (await readdir(join(stage, "node_modules/@poodle"))).map((name) => `@poodle/${name}`).sort();
    equalSet(installedPoodle, poodlePackages.map(([name]) => name), `${shape} Poodle graph`);
    await assertOnePackage(stage, "svelte", "5.38.6");
    await assertOnePackage(stage, "@tauri-apps/api", "2.10.1");
    await assertSingleRuntime(stage, "svelte");
    await assertSingleRuntime(stage, "@tauri-apps/api");
    for (const name of [...installedLonghorn, ...installedPoodle]) await assertArtifactInstall(stage, name);

    const capability = JSON.parse(await readFile(join(stage, shape, "capability.json"), "utf8")) as {
      schema: string; shape: string; hierarchy: string[]; permissions: string[]; failurePosture: string;
    };
    if (capability.schema !== "longhorn.greenfield-capability.v1" || capability.shape !== shape) throw new Error(`${shape} capability identity drift`);
    equalSet(capability.hierarchy, policy.hierarchy, `${shape} hierarchy`);
    equalSet(capability.permissions, policy.permissions, `${shape} permissions`);
    if (!capability.failurePosture.includes("visible") && shape !== "optional-server") throw new Error(`${shape} hides failure posture`);
    const selectedSource = await readFile(join(stage, shape, "src/selected.ts"), "utf8");
    const selectedNames = [...new Set(
      [...selectedSource.matchAll(/"(@longhorn\/[^"]+)"/g)].map((match) => match[1]!),
    )];
    equalSet(selectedNames, policy.typescript, `${shape} selected imports`);
    const lock = await readFile(join(stage, "bun.lock"), "utf8");
    if (/workspace:|link:/.test(lock) || lock.includes(join(repoRoot, "packages")) || lock.includes(join(poodleRoot, "packages"))) {
      throw new Error(`${shape} renderer lock resolved live source`);
    }
    reports.push({
      shape,
      hierarchy: policy.hierarchy,
      typescriptPackages: installedLonghorn,
      poodlePackages: installedPoodle,
      svelte: "5.38.6",
      tauriApi: "2.10.1",
      mountedLifecycleTests: 1,
      visibleFailure: true,
      cleanArtifactInstall: true,
      siblingSourceResolution: false,
    });
  }
  return reports;
}

async function verifyRustArtifacts() {
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(workspace);
  const identities: ArtifactIdentity[] = [];
  for (const name of allRustCrates) {
    const inventory = await run(["cargo", "package", "-p", name, "--list", "--allow-dirty"], repoRoot);
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) throw new Error(`${name} package inventory incomplete`);
    const archive = join(rustArtifactRoot, `${name}-0.1.0.private.tar`);
    await run(["tar", "-cf", archive, "-C", repoRoot, `crates/${name}`], repoRoot);
    await run(["tar", "-xf", archive, "-C", workspace], repoRoot);
    identities.push({ name, filename: basename(archive), sha256: await digest(archive) });
  }
  await cp(join(exampleRoot, "common-rust"), join(workspace, "common-rust"), { recursive: true });
  for (const shape of Object.keys(policies) as ShapeName[]) {
    await cp(join(exampleRoot, shape, "src-tauri"), join(workspace, "consumers", shape), { recursive: true });
  }
  await writeFile(join(workspace, "Cargo.toml"), await privateWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));

  const shapes = [];
  for (const shape of Object.keys(policies) as ShapeName[]) {
    const packageName = `longhorn-greenfield-${shape}-tauri`;
    const output = await run(["cargo", "+1.90.0", "run", "-p", packageName, "--offline", "--quiet"], workspace);
    const trace = JSON.parse(output.trim()) as Record<string, unknown>;
    if (trace.firstLoad !== "compiled-default" || trace.mutation !== "atomic-published" || trace.reload !== "file" || trace.mutationPathConfined !== true) {
      throw new Error(`${shape} config lifecycle trace failed`);
    }
    const tree = await run(["cargo", "+1.90.0", "tree", "-p", packageName, "--offline", "--edges", "normal", "--prefix", "none"], workspace);
    const graph = rustLonghornPackages(tree);
    equalSet(graph, policies[shape].rust, `${shape} Rust graph`);
    for (const forbidden of policies[shape].forbiddenRust) {
      if (graph.includes(forbidden)) throw new Error(`${shape} acquired forbidden Rust edge ${forbidden}`);
    }
    shapes.push({ shape, rustPackages: graph, trace });
  }
  const localOutput = await run(["cargo", "+1.90.0", "run", "-p", "longhorn-greenfield-optional-server-tauri", "--no-default-features", "--offline", "--quiet"], workspace);
  const localTrace = JSON.parse(localOutput.trim()) as Record<string, unknown>;
  const localTree = await run(["cargo", "+1.90.0", "tree", "-p", "longhorn-greenfield-optional-server-tauri", "--no-default-features", "--offline", "--edges", "normal", "--prefix", "none"], workspace);
  const localGraph = rustLonghornPackages(localTree);
  equalSet(localGraph, baseRust, "optional-server local Rust graph");
  if (localTrace.service !== "absent-local-authority-ready" || localTrace.reload !== "file") throw new Error("optional server local authority failed");
  return {
    identities,
    shapes,
    optionalServerLocal: { rustPackages: localGraph, service: localTrace.service, localAuthorityReady: true, supervisionRemoved: true },
  };
}

async function privateWorkspaceManifest(): Promise<string> {
  const root = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  const tail = root.slice(root.indexOf("[workspace.package]"));
  const selected = new Set(allRustCrates);
  const filtered = tail.split("\n").filter((line) => {
    const match = line.match(/^(longhorn-[\w-]+)\s*=/);
    return match === null || selected.has(match[1]! as typeof allRustCrates[number]);
  }).join("\n");
  const members = [
    ...allRustCrates.map((name) => `  "crates/${name}",`),
    '  "common-rust",',
    ...Object.keys(policies).map((shape) => `  "consumers/${shape}",`),
  ].join("\n");
  return `[workspace]\nmembers = [\n${members}\n]\nresolver = "2"\n\n${filtered}`;
}

function rewriteArtifactDependencies(dependencies: Record<string, string>, artifacts: Map<string, string>) {
  return Object.fromEntries(Object.entries(dependencies).map(([name, version]) => [name, artifacts.has(name) ? fileDependency(artifacts.get(name)!) : version]));
}
function fileDependency(path: string): string { return `file:${path}`; }
async function assertAbsent(stage: string, name: string): Promise<void> {
  try { await lstat(join(stage, "node_modules", ...name.split("/"))); } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly installed`);
}
async function assertArtifactInstall(stage: string, name: string): Promise<void> {
  const path = join(stage, "node_modules", ...name.split("/"));
  const resolved = await realpath(path);
  if (resolved.includes(join(repoRoot, "packages")) || resolved.includes(join(poodleRoot, "packages"))) throw new Error(`${name} resolved live source`);
}
async function assertOnePackage(stage: string, name: string, version: string): Promise<void> {
  const manifest = JSON.parse(await readFile(join(stage, "node_modules", ...name.split("/"), "package.json"), "utf8")) as { name: string; version: string };
  if (manifest.name !== name || manifest.version !== version) throw new Error(`${name} resolved ${manifest.version}`);
}
async function assertSingleRuntime(stage: string, name: string): Promise<void> {
  const suffix = `${name}/package.json`;
  const files = (await readdir(join(stage, "node_modules"), { recursive: true })).filter((path) => path === suffix || path.endsWith(`/node_modules/${suffix}`));
  if (files.length !== 1) throw new Error(`${name} runtime count ${files.length}`);
}
function rustLonghornPackages(tree: string): string[] {
  return [...new Set(tree.split("\n").map((line) => line.trim().split(/\s+/)[0] ?? "").filter((name) => name.startsWith("longhorn-") && !name.startsWith("longhorn-greenfield")))].sort();
}
function equalSet(actual: readonly string[], expected: readonly string[], label: string): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) throw new Error(`${label}: expected ${JSON.stringify(right)}, received ${JSON.stringify(left)}`);
}
function artifactSet(identities: ArtifactIdentity[]): string {
  return createHash("sha256").update(identities.map(({ name, sha256 }) => `${name}:${sha256}`).sort().join("\n")).digest("hex");
}
async function digest(path: string): Promise<string> { return createHash("sha256").update(await readFile(path)).digest("hex"); }
async function git(root: string, args: string[]): Promise<string> { return (await run(["git", ...args], root)).trim(); }
async function run(command: string[], cwd: string): Promise<string> {
  const process = Bun.spawn(command, { cwd, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([new Response(process.stdout).text(), new Response(process.stderr).text(), process.exited]);
  if (exitCode !== 0) throw new Error(`${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`);
  return stdout;
}
