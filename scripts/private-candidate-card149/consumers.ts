import { lstat, mkdir, readFile, readdir, realpath, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { CandidateArtifacts, GraphDefinition, GraphProof, PackageManifest } from "./types";
import {
  assert,
  digest,
  equal,
  equalSet,
  fileDependency,
  git,
  repoRoot,
  run,
  stableJson,
} from "./support";

const externalGraphs = [
  {
    name: "nucleus",
    repository: resolve(repoRoot, "../nucleus"),
    typescript: ["apps/desktop/package.json"],
    rust: ["apps/desktop/src-tauri/Cargo.toml"],
  },
  {
    name: "loophole",
    repository: resolve(repoRoot, "../loophole"),
    typescript: ["aura/package.json"],
    rust: [
      "aura/src-tauri/Cargo.toml",
      "echo/crates/echo-storage-profile/Cargo.toml",
      "pulse/Cargo.toml",
      "pulse/crates/pulse-history/Cargo.toml",
    ],
  },
  {
    name: "soundcheck",
    repository: resolve(repoRoot, "../soundcheck"),
    typescript: ["package.json"],
    rust: ["Cargo.toml", "src-tauri/Cargo.toml"],
  },
  {
    name: "bovine",
    repository: resolve(repoRoot, "../acowtancy/bovine-accelerator-desktop"),
    typescript: ["package.json"],
    rust: ["src-tauri/Cargo.toml"],
  },
  {
    name: "jetstream",
    repository: resolve(repoRoot, "../jetstream"),
    typescript: ["editor-ui/package.json"],
    rust: ["crates/jetstream-editor-tauri/Cargo.toml"],
  },
] as const;

const forbiddenRust: Record<string, string[]> = {
  minimal: ["longhorn-layout", "longhorn-surfaces", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
  nucleus: ["longhorn-surfaces", "longhorn-surfaces-config", "longhorn-transfer", "longhorn-surface-transfer", "longhorn-surface-windowing"],
  loophole: ["longhorn-bridge", "longhorn-operation", "longhorn-notifications", "longhorn-native-content"],
  soundcheck: ["longhorn-layout", "longhorn-surfaces", "longhorn-transfer", "longhorn-bridge", "longhorn-history"],
  bovine: ["longhorn-windowing", "longhorn-layout", "longhorn-surfaces", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
  jetstream: ["longhorn-config", "longhorn-settings", "longhorn-layout", "longhorn-surfaces", "longhorn-history", "longhorn-operation"],
  "optional-server": ["longhorn-layout", "longhorn-surfaces", "longhorn-history", "longhorn-native-content"],
};

export async function graphDefinitions(): Promise<GraphDefinition[]> {
  const greenfield = JSON.parse(await readFile(join(repoRoot, "fixtures/greenfield/card125/composition-matrix-v1.json"), "utf8")) as {
    shapes: Record<string, { typescript: string[]; rust: string[] }>;
  };
  const definitions: GraphDefinition[] = [
    {
      name: "minimal",
      repository: null,
      sourceCommit: null,
      sourceManifests: ["examples/greenfield-compositions/minimal"],
      typescript: greenfield.shapes.minimal!.typescript,
      rust: greenfield.shapes.minimal!.rust,
    },
  ];
  for (const graph of externalGraphs) {
    const sourceManifests = [...graph.typescript, ...graph.rust];
    const typescript = new Set<string>();
    for (const path of graph.typescript) {
      const manifest = JSON.parse(await readFile(join(graph.repository, path), "utf8")) as PackageManifest & {
        devDependencies?: Record<string, string>;
      };
      for (const name of Object.keys({ ...manifest.dependencies, ...manifest.devDependencies })) {
        if (name.startsWith("@longhorn/")) typescript.add(name);
      }
    }
    const rust = new Set<string>();
    for (const path of graph.rust) {
      const manifest = await readFile(join(graph.repository, path), "utf8");
      for (const match of manifest.matchAll(/^\s*(longhorn-[a-z0-9-]+)\s*=\s*/gm)) rust.add(match[1]!);
    }
    definitions.push({
      name: graph.name,
      repository: graph.repository,
      sourceCommit: await git(graph.repository, ["rev-parse", "HEAD"]),
      sourceManifests,
      typescript: [...typescript].sort(),
      rust: [...rust].sort(),
    });
  }
  definitions.push({
    name: "optional-server",
    repository: null,
    sourceCommit: null,
    sourceManifests: ["examples/greenfield-compositions/optional-server"],
    typescript: greenfield.shapes["optional-server"]!.typescript,
    rust: greenfield.shapes["optional-server"]!.rust,
  });
  return definitions;
}

export async function verifyConsumerGraphs(
  temporaryRoot: string,
  definitions: GraphDefinition[],
  artifacts: CandidateArtifacts,
  manifests: Map<string, { directory: string; manifest: PackageManifest }>,
  longhornCommit: string,
): Promise<{ proofs: GraphProof[]; rustLockSha256: string }> {
  const typescript = new Map<string, string[]>();
  for (const definition of definitions) {
    typescript.set(definition.name, await verifyTypescriptGraph(temporaryRoot, definition, artifacts, manifests));
  }
  const rust = await verifyRustGraphs(temporaryRoot, definitions, artifacts, longhornCommit);
  return {
    proofs: definitions.map((definition) => ({
      name: definition.name,
      sourceCommit: definition.sourceCommit,
      sourceManifests: definition.sourceManifests,
      typescriptPackages: typescript.get(definition.name)!,
      rustDirectPackages: definition.rust,
      rustResolvedPackages: rust.graphs.get(definition.name)!,
      oneSvelteRuntime: true,
      oneTauriApiRuntime: true,
      siblingResolution: false,
      lockedRust185: true,
    })),
    rustLockSha256: rust.lockSha256,
  };
}

async function verifyTypescriptGraph(
  temporaryRoot: string,
  definition: GraphDefinition,
  artifacts: CandidateArtifacts,
  manifests: Map<string, { manifest: PackageManifest }>,
): Promise<string[]> {
  const stage = join(temporaryRoot, "consumers-ts", definition.name);
  await mkdir(join(stage, "src"), { recursive: true });
  const expected = dependencyClosure(definition.typescript, manifests);
  const dependencies: Record<string, string> = {
    "@tauri-apps/api": "2.11.1",
    svelte: "5.56.8",
  };
  for (const name of definition.typescript) dependencies[name] = fileDependency(requiredPath(artifacts.typescript.paths, name));
  for (const [name, path] of artifacts.poodle.paths) dependencies[name] = fileDependency(path);
  const overrides: Record<string, string> = { "@tauri-apps/api": "2.11.1", svelte: "5.56.8" };
  for (const [name, path] of [...artifacts.typescript.paths, ...artifacts.poodle.paths]) overrides[name] = fileDependency(path);
  await writeFile(join(stage, "package.json"), stableJson({
    name: `longhorn-card149-${definition.name}`,
    private: true,
    type: "module",
    dependencies,
    devDependencies: { "@types/bun": "1.3.14", "svelte-check": "4.7.4", typescript: "6.0.3" },
    overrides,
  }));
  await writeFile(join(stage, "tsconfig.json"), stableJson({
    compilerOptions: {
      target: "ES2022",
      module: "ESNext",
      moduleResolution: "Bundler",
      strict: true,
      noEmit: true,
      skipLibCheck: true,
      allowImportingTsExtensions: true,
      types: ["bun"],
    },
    include: ["src/**/*.ts"],
  }));
  await writeFile(join(stage, "src/graph.ts"), `${definition.typescript.map((name, index) => `import * as package${index} from ${JSON.stringify(name)};`).join("\n")}\nexport const graph = [${definition.typescript.map((_, index) => `package${index}`).join(", ")}];\n`);
  await writeFile(join(stage, "src/assets.d.ts"), "declare module \"*.css\";\n");
  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "tsconfig.json"], stage);
  await run(["bun", "x", "svelte-check", "--tsconfig", "tsconfig.json"], stage);
  const installed = (await readdir(join(stage, "node_modules/@longhorn"))).map((name) => `@longhorn/${name}`).sort();
  equalSet(installed, expected, `${definition.name} TypeScript graph`);
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  assert(!/workspace:|link:/.test(lock), `${definition.name} renderer lock contains a source alias`);
  assert(!lock.includes(join(repoRoot, "packages")), `${definition.name} renderer resolved sibling Longhorn source`);
  for (const name of installed) {
    const resolved = await realpath(join(stage, "node_modules", ...name.split("/")));
    assert(!resolved.startsWith(join(repoRoot, "packages")), `${name} resolved sibling source`);
  }
  equal(await runtimeCount(stage, "svelte"), 1, `${definition.name} Svelte runtime count`);
  equal(await runtimeCount(stage, "@tauri-apps/api"), 1, `${definition.name} Tauri API runtime count`);
  return installed;
}

async function verifyRustGraphs(
  temporaryRoot: string,
  definitions: GraphDefinition[],
  artifacts: CandidateArtifacts,
  longhornCommit: string,
) {
  const workspace = join(temporaryRoot, "consumers-rust");
  await mkdir(workspace, { recursive: true });
  for (const path of artifacts.rust.paths.values()) await run(["tar", "-xzf", path, "-C", workspace]);
  const crateNames = artifacts.rust.identities.map(({ name }) => name);
  const root = await run(["git", "show", `${longhornCommit}:Cargo.toml`]);
  const tail = root.slice(root.indexOf("[workspace.package]"));
  const members = [...crateNames.map((name) => `crates/${name}`), ...definitions.map(({ name }) => `consumers/${name}`)];
  await writeFile(join(workspace, "Cargo.toml"), `[workspace]\nmembers = ${JSON.stringify(members, null, 2)}\nresolver = "2"\n\n${tail}`);
  for (const definition of definitions) {
    const consumer = join(workspace, "consumers", definition.name);
    await mkdir(join(consumer, "src"), { recursive: true });
    await writeFile(join(consumer, "Cargo.toml"), `[package]\nname = "longhorn-card149-${definition.name}"\nversion = "0.0.0"\nedition.workspace = true\n\n[dependencies]\n${definition.rust.map((name) => `${name}.workspace = true`).join("\n")}\n`);
    await writeFile(join(consumer, "src/main.rs"), "fn main() {}\n");
  }
  await writeFile(join(workspace, "Cargo.lock"), await run(["git", "show", `${longhornCommit}:Cargo.lock`]));
  await run(["cargo", "+1.90.0", "update", "-p", "tauri", "--precise", "2.11.5", "--offline"], workspace);
  const graphs = new Map<string, string[]>();
  for (const definition of definitions) {
    const packageName = `longhorn-card149-${definition.name}`;
    await run(["cargo", "+1.90.0", "check", "-p", packageName, "--locked", "--offline"], workspace);
    const tree = await run(["cargo", "+1.90.0", "tree", "-p", packageName, "--locked", "--offline", "--edges", "normal", "--prefix", "none"], workspace);
    const resolved = [...new Set(tree.split("\n").map((line) => line.trim().split(/\s+/)[0] ?? "").filter((name) => name.startsWith("longhorn-") && !name.startsWith("longhorn-card149-")))].sort();
    for (const required of definition.rust) assert(resolved.includes(required), `${definition.name} Rust graph lacks ${required}`);
    for (const forbidden of forbiddenRust[definition.name] ?? []) assert(!resolved.includes(forbidden), `${definition.name} Rust graph acquired ${forbidden}`);
    assert(tree.includes("tauri v2.11.5") || !resolved.some((name) => name.startsWith("longhorn-tauri-")), `${definition.name} did not resolve Tauri 2.11.5`);
    graphs.set(definition.name, resolved);
  }
  return { graphs, lockSha256: await digest(join(workspace, "Cargo.lock")) };
}

function dependencyClosure(selected: string[], manifests: Map<string, { manifest: PackageManifest }>): string[] {
  const result = new Set<string>();
  const pending = [...selected];
  while (pending.length > 0) {
    const name = pending.pop()!;
    if (result.has(name)) continue;
    const manifest = manifests.get(name)?.manifest;
    assert(manifest, `unknown TypeScript package ${name}`);
    result.add(name);
    for (const dependency of Object.keys(manifest.dependencies ?? {})) {
      if (dependency.startsWith("@longhorn/")) pending.push(dependency);
    }
    for (const peer of Object.keys(manifest.peerDependencies ?? {})) {
      if (peer.startsWith("@longhorn/") && manifest.peerDependenciesMeta?.[peer]?.optional !== true) pending.push(peer);
    }
  }
  return [...result].sort();
}

async function runtimeCount(stage: string, packageName: string): Promise<number> {
  const suffix = `${packageName}/package.json`;
  return (await readdir(join(stage, "node_modules"), { recursive: true })).filter((path) => path === suffix || path.endsWith(`/node_modules/${suffix}`)).length;
}

function requiredPath(paths: Map<string, string>, name: string): string {
  const path = paths.get(name);
  assert(path, `candidate has no artifact for ${name}`);
  return path;
}

export async function assertGraphSourcesClean(definitions: GraphDefinition[]): Promise<void> {
  for (const definition of definitions) {
    if (!definition.repository || !definition.sourceCommit) continue;
    equal(await git(definition.repository, ["rev-parse", "HEAD"]), definition.sourceCommit, `${definition.name} source commit`);
    const status = await git(definition.repository, ["status", "--porcelain", "--", ...definition.sourceManifests]);
    assert(!status, `${definition.name} selected manifests are dirty:\n${status}`);
    for (const path of definition.sourceManifests) await lstat(join(definition.repository, path));
  }
}
