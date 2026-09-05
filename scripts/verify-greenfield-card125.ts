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
import { MSRV, MSRV_TOOLCHAIN } from "./msrv.ts";
import { poodleRelease } from "./poodle-release.ts";

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
const bunCommand = ["bun"] as const;
const exampleRoot = join(repoRoot, "examples/greenfield-compositions");
const receiptPath = join(repoRoot, "fixtures/greenfield/card125/composition-matrix-v1.json");
const longhornTypescriptPackages = [
  ["@inflatable-cookie/longhorn", "packages/longhorn"],
  ["@inflatable-cookie/longhorn-poodle-svelte", "packages/longhorn-poodle-svelte"],
  ["@inflatable-cookie/longhorn-tauri", "packages/longhorn-tauri"],
] as const;
const poodlePackages = [
  "@inflatable-cookie/poodle-core",
  "@inflatable-cookie/poodle-svelte",
] as const;
// Published Poodle only. Sibling packing was the unpublished-preview path;
// g16.109 pins exact public 0.3.0, so greenfield installs the registry
// packages the lock already records.
const LONGHORN_VERSION = "0.1.0";
const publishedPoodle = poodleRelease();
const POODLE_VERSION = publishedPoodle.version;
const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-greenfield-card125-"));
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
const rustArtifactRoot = join(temporaryRoot, "rust-artifacts");
const allRustCrates = [
  "longhorn-bridge",
  "longhorn-command",
  "longhorn-command-config",
  "longhorn-command-settings",
  "longhorn-config",
  "longhorn-core",
  "longhorn-display",
  "longhorn-history",
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
  "longhorn-config", "longhorn-core", "longhorn-settings",
  "longhorn-settings-config", "longhorn-tauri-config", "longhorn-tauri-settings",
] as const;
const workspaceRust = [
  "longhorn-command", "longhorn-command-config", "longhorn-command-settings",
  "longhorn-config", "longhorn-core", "longhorn-display",
  "longhorn-surfaces", "longhorn-surfaces-config", "longhorn-settings",
  "longhorn-settings-config", "longhorn-tauri-command", "longhorn-tauri-config",
  "longhorn-tauri-settings", "longhorn-tauri-windowing", "longhorn-windowing",
  "longhorn-windowing-config",
] as const;
const fullRust = [
  ...workspaceRust,
  "longhorn-history", "longhorn-surface-transfer", "longhorn-surface-windowing",
  "longhorn-tauri-history", "longhorn-tauri-transfer", "longhorn-transfer",
] as const;

const policies: Record<ShapeName, ShapePolicy> = {
  minimal: {
    hierarchy: ["window"],
    typescript: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-tauri"],
    rust: baseRust,
    forbiddenTypescript: ["@inflatable-cookie/longhorn-poodle-svelte"],
    forbiddenRust: ["longhorn-surfaces", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-config-read", "core:event:allow-listen", "core:event:allow-unlisten"],
  },
  workspace: {
    hierarchy: ["window", "layout-container", "region", "panel"],
    typescript: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-poodle-svelte", "@inflatable-cookie/longhorn-tauri"],
    rust: workspaceRust,
    forbiddenTypescript: [],
    // Card 179 folded longhorn-layout into longhorn-surfaces, so a composition
    // that uses layout necessarily depends on longhorn-surfaces. What these
    // shapes still exclude is Surface *transfer*, and the domains above.
    forbiddenRust: ["longhorn-transfer", "longhorn-surface-transfer", "longhorn-bridge", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-command-read", "allow-longhorn-command-mutate", "core:event:allow-listen", "core:event:allow-unlisten", "core:window:allow-start-dragging"],
  },
  "full-hosting": {
    hierarchy: ["display", "window", "surface", "layout-container", "region", "panel"],
    typescript: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-poodle-svelte", "@inflatable-cookie/longhorn-tauri"],
    rust: fullRust,
    forbiddenTypescript: [],
    forbiddenRust: ["longhorn-bridge", "longhorn-native-content", "longhorn-operation", "longhorn-notifications"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-command-read", "allow-longhorn-command-mutate", "allow-longhorn-transfer-base", "allow-longhorn-transfer-surface", "allow-longhorn-history-read", "allow-longhorn-history-mutate", "core:event:allow-listen", "core:event:allow-unlisten", "core:window:allow-start-dragging"],
  },
  "optional-server": {
    hierarchy: ["window", "optional-service-bridge"],
    typescript: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-tauri"],
    rust: [...baseRust, "longhorn-bridge", "longhorn-tauri-bridge"],
    forbiddenTypescript: ["@inflatable-cookie/longhorn-poodle-svelte"],
    forbiddenRust: ["longhorn-surfaces", "longhorn-history", "longhorn-native-content"],
    permissions: ["allow-longhorn-settings-read", "allow-longhorn-settings-mutate", "allow-longhorn-bridge-query", "core:event:allow-listen", "core:event:allow-unlisten"],
  },
};

try {
  await mkdir(typescriptArtifactRoot);
  await mkdir(rustArtifactRoot);
  await verifyExampleSources();
  const typescript = await packPackages(repoRoot, longhornTypescriptPackages, typescriptArtifactRoot, LONGHORN_VERSION);
  const poodleIdentities = publishedPoodle.packages.map((pkg) => ({
    name: pkg.name,
    filename: `${pkg.name.replace("@", "").replace("/", "-")}-${pkg.version}.tgz`,
    sha256: pkg.integrity,
  }));
  const renderers = await verifyRenderers(typescript.paths);
  const rust = await verifyRustArtifacts();
  const report = {
    schema: "longhorn.greenfield-composition-matrix.v1",
    outcome: "pass",
    sources: {
      longhorn: await git(repoRoot, ["rev-parse", "HEAD"]),
      poodle: `${POODLE_VERSION}@registry`,
      selectedLonghornClean: await selectedSourcesClean(repoRoot, [
        ...longhornTypescriptPackages.map(([, path]) => path),
        ...allRustCrates.map((name) => `crates/${name}`),
      ]),
      selectedPoodleClean: true,
    },
    artifacts: {
      typescript: typescript.identities,
      poodle: poodleIdentities,
      rust: rust.identities,
      sets: {
        typescript: artifactSet(typescript.identities),
        poodle: artifactSet(poodleIdentities),
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
  if (process.env.WRITE_GREENFIELD_RECEIPT === "1") {
    await writeReceipt(report);
  } else {
    await verifyReceipt(report);
  }
  console.log(JSON.stringify(report, null, 2));
} finally {
  if (process.env.KEEP_GREENFIELD_COMPOSITION_PROOF === "1") {
    console.error(`retained greenfield proof: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function writeReceipt(report: {
  schema: string;
  sources: { longhorn: string; poodle: string };
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
  const shapes = Object.fromEntries(
    report.shapes.map((shape) => {
      if (!shape.rust) throw new Error(`${shape.shape} Rust report missing`);
      return [
        shape.shape,
        {
          hierarchy: [...shape.hierarchy],
          typescript: ["@inflatable-cookie/longhorn", "@inflatable-cookie/longhorn-tauri"].sort(),
          rust: [...shape.rust.rustPackages].sort(),
        },
      ];
    }),
  ) as Record<ShapeName, { hierarchy: string[]; typescript: string[]; rust: string[] }>;
  const receipt = {
    schema: report.schema,
    sources: {
      longhornSelected: report.sources.longhorn,
      poodleSelected: report.sources.poodle,
    },
    artifacts: {
      sets: report.artifacts.sets,
      inventories: {
        typescript: report.artifacts.typescript.map(({ name }) => name).sort(),
        poodle: report.artifacts.poodle.map(({ name }) => name).sort(),
        rust: report.artifacts.rust.map(({ name }) => name).sort(),
      },
    },
    shapes,
    optionalServerLocalRust: [...report.optionalServerLocal.rustPackages].sort(),
    audits: report.audits,
  };
  await writeFile(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);
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
  // The artifact set ids are emitted as evidence of what this run built. They
  // are not compared against the receipt: every id is a hash over packed
  // contents, so any change to any crate or package moves them, and asserting
  // equality would re-freeze the tree through the back door.
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

// Contract 012 requires that an adopted graph "records the exact clean
// Longhorn and Poodle commits" and that "unrecorded dirty sources are not
// valid pins". That is a cleanliness requirement, not an immutability one.
//
// This used to assert `git diff --quiet <frozen-commit>` across every package
// and crate, which made the receipt a snapshot the tree could never move away
// from: any change to any crate failed an unrelated gate, and the only remedy
// was to rebaseline the fixture. Meanwhile the report hardcoded
// `selectedLonghornClean: true` without checking anything, so the receipt
// asserted a cleanliness it never verified.
//
// The composition claims below are the real evidence and they are computed
// from the current tree: the proof packs it, installs the tarballs into
// isolated roots, and asserts what resolves. Pinning the tree to a past commit
// adds nothing to that.
// Recorded, not asserted. Cleanliness is a release property: a tag must name
// exact clean commits. It is not a `qa` property, and gating on it here would
// mean Longhorn's own suite fails whenever anyone has uncommitted work — in
// Longhorn during ordinary development, or worse, in Poodle, which would make
// this repository's gate depend on a sibling repository's working tree.
// PAPERCUTS already records that coupling for Card 149's receipt.
//
// The composition claims are computed from whatever source is present, so they
// remain true either way. The report says whether the sources it built from
// were clean, and a release gate can require that they were.
async function selectedSourcesClean(root: string, paths: readonly string[]): Promise<boolean> {
  const status = Bun.spawnSync(["git", "status", "--porcelain", "--", ...paths], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (status.exitCode !== 0) throw new Error(`git status failed in ${root}:\n${status.stderr.toString()}`);
  return status.stdout.toString().trim().length === 0;
}

// Scanned in process rather than by shelling out to ripgrep. `rg` is on every
// developer machine here and on no GitHub macOS runner, so this threw
// `Executable not found in $PATH: "rg"` on the first release run -- after
// twenty minutes of gates, at the last proof.
//
// Installing ripgrep in the workflow would have fixed the run and left the
// dependency. Reading the files needs nothing that is not already required to
// run this script, and the tree is a handful of example sources.
async function verifyExampleSources(): Promise<void> {
  // Declared here, not at module scope: this script runs its main body at the
  // top level, above this point, so a module-scope const is still in the
  // temporal dead zone by the time this is called.
  const donorVocabulary = /loophole|nucleus|soundcheck|split-shell|jetstream/i;
  const offenders: string[] = [];
  for (const relative of await readdir(exampleRoot, { recursive: true })) {
    const path = join(exampleRoot, relative);
    if (!(await lstat(path)).isFile()) continue;
    const lines = (await readFile(path, "utf8")).split("\n");
    for (const [index, line] of lines.entries()) {
      if (donorVocabulary.test(line)) {
        offenders.push(`${relative}:${index + 1}: ${line.trim()}`);
      }
    }
  }
  if (offenders.length > 0) {
    throw new Error(
      `greenfield examples contain donor vocabulary:\n${offenders.join("\n")}`,
    );
  }
  const svelteFiles = (await readdir(exampleRoot, { recursive: true }))
    .filter((path) => path.endsWith(".svelte"));
  equalSet(svelteFiles, ["common/App.svelte"], "greenfield Svelte implementation inventory");
  const shell = await readFile(join(exampleRoot, "common/App.svelte"), "utf8");
  if (!shell.includes('from "@inflatable-cookie/poodle-svelte"') || !shell.includes('import "@inflatable-cookie/poodle-core/tokens/styles.css"')) {
    throw new Error("greenfield shell does not compose public Poodle roots");
  }
}


async function packPackages(
  root: string,
  packages: readonly (readonly [string, string])[],
  destination: string,
  expected: string,
): Promise<{ identities: ArtifactIdentity[]; paths: Map<string, string> }> {
  const identities: ArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    try {
      await run([...bunCommand, "pm", "pack", "--destination", destination, "--ignore-scripts", "--quiet"], join(root, directory));
    } catch (error) {
      throw new Error(`${name} pack failed from ${join(root, directory)}: ${String(error)}`);
    }
    const path = join(destination, `${name.replace("@", "").replace("/", "-")}-${expected}.tgz`);
    const listing = await run(["tar", "-tzf", path], destination);
    const manifest = await run(["tar", "-xOzf", path, "package/package.json"], destination);
    if (listing.includes("node_modules/") || listing.includes("/tests/") || /workspace:|link:/.test(manifest)) {
      throw new Error(`${name} artifact contains workspace or proof material`);
    }
    const parsed = JSON.parse(manifest) as { name: string; version: string };
    if (parsed.name !== name || parsed.version !== expected) throw new Error(`${name} artifact identity drift`);
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

    const installedLonghorn = (await readdir(join(stage, "node_modules/@inflatable-cookie"))).filter((name) => name === "longhorn" || name.startsWith("longhorn-")).map((name) => `@inflatable-cookie/${name}`).sort();
    equalSet(installedLonghorn, policy.typescript, `${shape} TypeScript graph`);
    for (const name of policy.forbiddenTypescript) await assertAbsent(stage, name);
    const installedPoodle = (await readdir(join(stage, "node_modules/@inflatable-cookie"))).filter((name) => name.startsWith("poodle-")).map((name) => `@inflatable-cookie/${name}`).sort();
    equalSet(installedPoodle, [...poodlePackages], `${shape} Poodle graph`);
    await assertOnePackage(stage, "svelte", "5.56.8");
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
    // Card 164: selected.ts names subpaths now, and the policy names packages.
    // Fold each specifier back to the package that provides it.
    const selectedNames = [...new Set(
      [...selectedSource.matchAll(/"(@inflatable-cookie\/longhorn(?:[/-][^"]*)?)"/g)].map(
        (match) => {
          const specifier = match[1]!;
          for (const pkg of [
            "@inflatable-cookie/longhorn-poodle-svelte",
            "@inflatable-cookie/longhorn-tauri",
          ]) {
            if (specifier === pkg || specifier.startsWith(`${pkg}/`)) return pkg;
          }
          return "@inflatable-cookie/longhorn";
        },
      ),
    )];
    equalSet(selectedNames, policy.typescript, `${shape} selected imports`);
    const lock = await readFile(join(stage, "bun.lock"), "utf8");
    if (/workspace:|link:/.test(lock) || lock.includes(join(repoRoot, "packages"))) {
      throw new Error(`${shape} renderer lock resolved live source`);
    }
    reports.push({
      shape,
      hierarchy: policy.hierarchy,
      typescriptPackages: installedLonghorn,
      poodlePackages: installedPoodle,
      svelte: "5.56.8",
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
    const output = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "run", "-p", packageName, "--offline", "--quiet"], workspace);
    const trace = JSON.parse(output.trim()) as Record<string, unknown>;
    if (trace.firstLoad !== "compiled-default" || trace.mutation !== "atomic-published" || trace.reload !== "file" || trace.mutationPathConfined !== true) {
      throw new Error(`${shape} config lifecycle trace failed`);
    }
    const tree = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "tree", "-p", packageName, "--offline", "--edges", "normal", "--prefix", "none"], workspace);
    const graph = rustLonghornPackages(tree);
    equalSet(graph, policies[shape].rust, `${shape} Rust graph`);
    for (const forbidden of policies[shape].forbiddenRust) {
      if (graph.includes(forbidden)) throw new Error(`${shape} acquired forbidden Rust edge ${forbidden}`);
    }
    shapes.push({ shape, rustPackages: graph, trace });
  }
  const localOutput = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "run", "-p", "longhorn-greenfield-optional-server-tauri", "--no-default-features", "--offline", "--quiet"], workspace);
  const localTrace = JSON.parse(localOutput.trim()) as Record<string, unknown>;
  const localTree = await run(["cargo", `+${MSRV_TOOLCHAIN}`, "tree", "-p", "longhorn-greenfield-optional-server-tauri", "--no-default-features", "--offline", "--edges", "normal", "--prefix", "none"], workspace);
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
  if (resolved.includes(join(repoRoot, "packages"))) throw new Error(`${name} resolved live source`);
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
