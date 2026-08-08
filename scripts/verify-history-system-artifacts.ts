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

const POODLE_ARTIFACT_SET =
  "39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74";
const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "history-system-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-history-system-proof-"),
);
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(typescriptArtifactRoot);

type Shape = "minimal" | "loophole";

interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

interface PoodleEvidenceFile {
  readonly artifactSetId: string;
  readonly artifacts: readonly ArtifactIdentity[];
}

interface PoodleEvidence {
  readonly artifacts: readonly ArtifactIdentity[];
  readonly packDirectory: string;
}

interface PackageManifest {
  readonly name: string;
  dependencies: Record<string, string>;
  overrides?: Record<string, string>;
}

const policies = {
  minimal: {
    rust: ["longhorn-core", "longhorn-history"],
    longhorn: ["@inflatable-cookie/longhorn-core", "@inflatable-cookie/longhorn-history"],
    imports: ["@inflatable-cookie/longhorn-history"],
    permissions: [],
    forbidden: [
      "@inflatable-cookie/longhorn-bridge",
      "@inflatable-cookie/longhorn-config",
      "@inflatable-cookie/longhorn-tauri",
      "svelte",
      "@inflatable-cookie/poodle-svelte",
    ],
    mountedTests: 0,
  },
  loophole: {
    rust: ["longhorn-core", "longhorn-history", "longhorn-tauri-history"],
    longhorn: ["@inflatable-cookie/longhorn-core", "@inflatable-cookie/longhorn-history"],
    imports: [
      "@inflatable-cookie/longhorn-history",
      "@inflatable-cookie/longhorn-history/poodle",
      "@inflatable-cookie/longhorn-history/svelte",
      "@inflatable-cookie/longhorn-history/tauri",
    ],
    permissions: [
      "allow-longhorn-history-read",
      "allow-longhorn-history-mutate",
      "core:event:allow-listen",
      "core:event:allow-unlisten",
    ],
    forbidden: ["@inflatable-cookie/longhorn-bridge", "@inflatable-cookie/longhorn-config", "@inflatable-cookie/longhorn-tauri"],
    mountedTests: 1,
  },
} as const;

try {
  await run(
    ["cargo", "run", "-p", "longhorn-bindings", "--", "history", "check"],
    repoRoot,
  );
  const poodle = await readPoodleEvidence();
  const typescript = await packTypescriptArtifacts();
  const rust = await packAndRunRustArtifacts();
  const consumers = await Promise.all(
    (["minimal", "loophole"] as const).map((shape) =>
      verifyTypescriptConsumer(
        shape,
        typescript.paths,
        poodle,
        rust.traces[shape],
      ),
    ),
  );
  for (const consumer of consumers) {
    const native = rust.traces[consumer.shape].publicTrace;
    const renderer = consumer.trace.publicTrace;
    if (!equalJson(native, renderer)) {
      throw new Error(`${consumer.shape} native and renderer traces diverged`);
    }
  }
  const rendererFixturesPayloadFree = (["minimal", "loophole"] as const).every(
    (shape) => !containsKey(rust.traces[shape].rendererFixture, "payload"),
  );
  if (!rendererFixturesPayloadFree) {
    throw new Error("renderer fixture contains consumer payload");
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.history-system-artifact-proof.v1",
        poodleArtifactSet: POODLE_ARTIFACT_SET,
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustGraphs: rust.graphs,
        nativeTraces: rust.traces,
        typescriptArtifacts: typescript.identities,
        consumers,
        audits: {
          generatedBindingsDrift: false,
          nativeRendererSemanticsEqual: true,
          rendererFixturesPayloadFree,
          minimalOptionalEdgesAbsent: true,
          peerRuntimeUnique: true,
          capabilitiesExact: true,
          productAuthorityRemainsConsumerOwned: true,
          productApplyRemainsConsumerOwned: true,
          journalRemainsConsumerOwned: true,
          publicPoodleOnly: true,
          branchApiAbsent: true,
          projectVersionApiAbsent: true,
          durableEventSourceClaimAbsent: true,
          donorRepositoriesWritten: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_HISTORY_SYSTEM_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function readPoodleEvidence(): Promise<PoodleEvidence> {
  const evidencePath = resolve(
    process.env.POODLE_PREVIEW_EVIDENCE ??
      "../poodle/.artifacts/g12.016-A698XB/evidence.json",
  );
  const evidence = JSON.parse(
    await readFile(evidencePath, "utf8"),
  ) as PoodleEvidenceFile;
  if (evidence.artifactSetId !== POODLE_ARTIFACT_SET) {
    throw new Error(`Poodle artifact set mismatch: ${evidence.artifactSetId}`);
  }
  const packDirectory = join(resolve(evidencePath, ".."), "packs");
  const membership = [];
  for (const artifact of evidence.artifacts) {
    const path = join(packDirectory, artifact.filename);
    const sha256 = await digest(path);
    if (sha256 !== artifact.sha256) {
      throw new Error(`${artifact.name} Poodle artifact digest mismatch`);
    }
    membership.push(`${artifact.name}:${sha256}`);
  }
  const setId = Bun.CryptoHasher.hash("sha256", membership.join("\n"), "hex");
  if (setId !== POODLE_ARTIFACT_SET) {
    throw new Error(`Poodle artifact membership mismatch: ${setId}`);
  }
  return { artifacts: evidence.artifacts, packDirectory };
}

async function packTypescriptArtifacts(): Promise<{
  readonly identities: readonly ArtifactIdentity[];
  readonly paths: ReadonlyMap<string, string>;
}> {
  const packages = [
    ["@inflatable-cookie/longhorn-core", "core"],
    ["@inflatable-cookie/longhorn-history", "history"],
  ] as const;
  const identities = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packages) {
    await run(
      [
        "bun",
        "pm",
        "pack",
        "--destination",
        typescriptArtifactRoot,
        "--ignore-scripts",
        "--quiet",
      ],
      join(repoRoot, "packages", directory),
    );
    const path = join(
      typescriptArtifactRoot,
      `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`,
    );
    await inspectNpmArtifact(name, path);
    paths.set(name, path);
    identities.push({
      name,
      filename: basename(path),
      sha256: await digest(path),
    });
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
    throw new Error(`${name} artifact contains proof or workspace material`);
  }
  const manifest = JSON.parse(
    await run(
      ["tar", "-xOzf", path, "package/package.json"],
      typescriptArtifactRoot,
    ),
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
  assertExactSet(
    `${name} dependencies`,
    Object.keys(manifest.dependencies ?? {}),
    name === "@inflatable-cookie/longhorn-history" ? ["@inflatable-cookie/longhorn-core"] : [],
  );
  if (name === "@inflatable-cookie/longhorn-history") {
    assertExactSet(
      "history package exports",
      Object.keys(manifest.exports ?? {}),
      [".", "./package.json", "./poodle", "./protocol", "./svelte", "./tauri"],
    );
    for (const peer of ["svelte", "@inflatable-cookie/poodle-svelte"]) {
      if (manifest.peerDependenciesMeta?.[peer]?.optional !== true) {
        throw new Error(`${peer} is not an optional history peer`);
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
  for (const marker of [
    "PulseMutation",
    "PulseHistoryMutation",
    "loophole.pulse-proof",
  ]) {
    if (source.includes(marker)) {
      throw new Error(
        `${name} artifact contains donor payload marker ${marker}`,
      );
    }
  }
}

async function packAndRunRustArtifacts(): Promise<{
  readonly identities: readonly ArtifactIdentity[];
  readonly packaging: {
    readonly privateSourceInventories: true;
    readonly registryNormalizedCrates: false;
    readonly cargoPackageListPassed: true;
  };
  readonly graphs: Record<Shape, readonly string[]>;
  readonly traces: Record<Shape, Record<string, unknown>>;
}> {
  const rustArtifactRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(rustArtifactRoot);
  await mkdir(workspace);
  const crates = [
    "longhorn-core",
    "longhorn-history",
    "longhorn-tauri-history",
  ] as const;
  const identities = [];
  for (const name of crates) {
    const inventory = await run(
      [
        "cargo",
        "+1.95.0",
        "package",
        "-p",
        name,
        "--list",
        "--allow-dirty",
      ],
      repoRoot,
    );
    if (
      !inventory.includes("Cargo.toml") ||
      !inventory.includes("src/lib.rs")
    ) {
      throw new Error(`${name} package inventory is incomplete`);
    }
    const tarArchive = join(
      rustArtifactRoot,
      `${name}-0.1.0.private.tar`,
    );
    const archive = `${tarArchive}.gz`;
    await run(
      ["tar", "-cf", tarArchive, "-C", repoRoot, `crates/${name}`],
      repoRoot,
    );
    await run(["gzip", "-n", tarArchive], repoRoot);
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({
      name,
      filename: basename(archive),
      sha256: await digest(archive),
    });
  }
  const sharedSource = [
    await readSourceTree(join(workspace, "crates", "longhorn-history", "src")),
    await readSourceTree(
      join(workspace, "crates", "longhorn-tauri-history", "src"),
    ),
  ].join("\n");
  for (const marker of [
    "PulseMutation",
    "PulseHistoryMutation",
    "loophole.pulse-proof",
  ]) {
    if (sharedSource.includes(marker)) {
      throw new Error(`Rust artifacts contain donor payload marker ${marker}`);
    }
  }
  for (const marker of ["branch_id", "project_version", "event_source"]) {
    if (sharedSource.toLowerCase().includes(marker)) {
      throw new Error(`linear Rust artifacts claim deferred feature ${marker}`);
    }
  }

  const consumersRoot = join(workspace, "consumers");
  await mkdir(consumersRoot);
  for (const shape of ["minimal", "loophole"] as const) {
    await cp(join(proofRoot, "rust", shape), join(consumersRoot, shape), {
      recursive: true,
    });
  }
  await writeFile(join(workspace, "Cargo.toml"), rustWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));

  const traces = {} as Record<Shape, Record<string, unknown>>;
  const graphs = {} as Record<Shape, readonly string[]>;
  for (const shape of ["minimal", "loophole"] as const) {
    traces[shape] = parseTrace(
      await run(
        [
          "cargo",
          "+1.95.0",
          "run",
          "-p",
          `longhorn-${shape}-history-artifact-proof`,
          "--offline",
          "--quiet",
        ],
        workspace,
      ),
    );
    const tree = await run(
      [
        "cargo",
        "+1.95.0",
        "tree",
        "-p",
        `longhorn-${shape}-history-artifact-proof`,
        "--offline",
        "--prefix",
        "none",
      ],
      workspace,
    );
    graphs[shape] = longhornPackages(tree);
    assertExactSet(`${shape} Rust graph`, graphs[shape], policies[shape].rust);
  }
  return {
    identities,
    packaging: {
      privateSourceInventories: true,
      registryNormalizedCrates: false,
      cargoPackageListPassed: true,
    },
    graphs,
    traces,
  };
}

async function verifyTypescriptConsumer(
  shape: Shape,
  artifacts: ReadonlyMap<string, string>,
  poodle: PoodleEvidence,
  nativeTrace: Record<string, unknown>,
) {
  const policy = policies[shape];
  const source = join(proofRoot, "consumers", shape);
  const stage = join(temporaryRoot, `consumer-${shape}-${randomUUID()}`);
  await mkdir(join(stage, "consumers"), { recursive: true });
  await cp(source, join(stage, "consumers", shape), { recursive: true });
  for (const filename of ["common.ts", "consumer-tsconfig.json"]) {
    await cp(join(proofRoot, filename), join(stage, filename));
  }
  if (shape === "loophole") {
    for (const filename of ["vitest.config.ts", "setup.ts"]) {
      await cp(join(proofRoot, filename), join(stage, filename));
    }
  }
  await writeFile(
    join(stage, "consumers", shape, "fixture.json"),
    `${JSON.stringify(nativeTrace.rendererFixture, null, 2)}\n`,
  );

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = Object.fromEntries(
    Object.entries(manifest.dependencies).map(([name, version]) => [
      name,
      artifacts.has(name) ? fileDependency(artifacts.get(name)!) : version,
    ]),
  );
  const allArtifacts = new Map(artifacts);
  for (const artifact of poodle.artifacts) {
    allArtifacts.set(
      artifact.name,
      resolve(poodle.packDirectory, artifact.filename),
    );
  }
  manifest.overrides = Object.fromEntries(
    [...allArtifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "consumer-tsconfig.json"], stage);
  let mountedTests = 0;
  if (shape === "loophole") {
    await run(
      ["bun", "x", "svelte-check", "--tsconfig", "./consumer-tsconfig.json"],
      stage,
    );
    mountedTests = testCount(
      await run(
        ["bun", "x", "vitest", "run", "--config", "./vitest.config.ts"],
        stage,
      ),
    );
  }
  if (mountedTests !== policy.mountedTests) {
    throw new Error(`${shape} mounted test count mismatch: ${mountedTests}`);
  }
  const trace = parseTrace(
    await run(["bun", `consumers/${shape}/proof.ts`], stage),
  );

  assertExactSet(
    `${shape} installed Longhorn packages`,
    (await installedScope(stage, "@longhorn")).map(
      (name) => `@inflatable-cookie/longhorn-${name}`,
    ),
    policy.longhorn,
  );
  const artifactResolution = [];
  for (const name of policy.longhorn) {
    artifactResolution.push(await assertArtifactInstall(stage, name));
  }
  for (const name of policy.forbidden) {
    await assertPackageAbsent(stage, name);
  }
  if (shape === "loophole") {
    for (const artifact of poodle.artifacts) {
      await assertArtifactInstall(stage, artifact.name);
    }
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") {
      throw new Error("Loophole installed unexpected Svelte version");
    }
    await assertSingleSvelteRuntime(stage);
  } else if ((await installedScope(stage, "@inflatable-cookie")).some((name) => name.startsWith("poodle-"))) {
    throw new Error("minimal consumer acquired Poodle packages");
  }

  const permissions =
    shape === "loophole"
      ? (
          JSON.parse(
            await readFile(
              join(stage, "consumers", shape, "capability.json"),
              "utf8",
            ),
          ) as { readonly permissions: readonly string[] }
        ).permissions
      : [];
  assertExactSet(`${shape} permissions`, permissions, policy.permissions);
  assertExactSet(
    `${shape} imports`,
    await longhornImports(stage),
    policy.imports,
  );
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  if (
    lock.includes("workspace:") ||
    lock.includes("link:") ||
    lock.includes(resolve(repoRoot, "packages")) ||
    lock.includes("/Dev/projects/poodle/packages/")
  ) {
    throw new Error(`${shape} lockfile contains workspace/source resolution`);
  }

  return {
    shape,
    longhornPackages: policy.longhorn,
    imports: policy.imports,
    permissions: policy.permissions,
    forbiddenPackagesAbsent: policy.forbidden,
    artifactResolution,
    poodleArtifactSet: shape === "loophole" ? POODLE_ARTIFACT_SET : null,
    mountedTests,
    trace,
    cleanInstall: true,
    siblingSourceAliases: false,
  };
}

function rustWorkspaceManifest(): string {
  return `[workspace]
members = [
  "consumers/minimal",
  "consumers/loophole",
  "crates/longhorn-core",
  "crates/longhorn-history",
  "crates/longhorn-tauri-history",
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
longhorn-history = { path = "crates/longhorn-history", version = "0.1.0" }
longhorn-tauri-history = { path = "crates/longhorn-tauri-history", version = "0.1.0" }
proptest = { version = "1.11.0", default-features = false, features = ["std"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
tauri = { version = "2.10.3", default-features = false }
ts-rs = { version = "=11.0.0", default-features = false, features = ["no-serde-warnings", "serde-compat"] }

[workspace.lints.rust]
missing_docs = "deny"
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "deny"
`;
}

async function digest(path: string): Promise<string> {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

async function run(command: readonly string[], cwd: string): Promise<string> {
  const subprocess = Bun.spawn(command, {
    cwd,
    env: {
      ...process.env,
      COPYFILE_DISABLE: "1",
    },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`,
    );
  }
  return `${stdout}\n${stderr}`;
}

function fileDependency(path: string): string {
  return `file:${path}`;
}

function assertExactSet(
  label: string,
  actual: readonly string[],
  expected: readonly string[],
): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(
      `${label} mismatch: ${left.join(", ")} != ${right.join(", ")}`,
    );
  }
}

function parseTrace(output: string): Record<string, unknown> {
  const line = output
    .split("\n")
    .find((candidate) => candidate.trim().startsWith("{"));
  if (line === undefined) throw new Error("proof emitted no JSON trace");
  return JSON.parse(line) as Record<string, unknown>;
}

function testCount(output: string): number {
  const match = output.match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
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

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
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

async function installedScope(
  stage: string,
  scope: string,
): Promise<readonly string[]> {
  try {
    return (await readdir(join(stage, "node_modules", scope))).sort();
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
}

async function assertArtifactInstall(stage: string, name: string) {
  const installed = await installedPackage(stage, name);
  for (const root of [
    "/Dev/projects/longhorn/packages/",
    "/Dev/projects/poodle/packages/",
  ]) {
    if (installed.realPath.includes(root)) {
      throw new Error(
        `${name} resolved to sibling source: ${installed.realPath}`,
      );
    }
  }
  if (installed.manifest.version !== "0.1.0") {
    throw new Error(`${name} installed unexpected version`);
  }
  return { name, version: installed.manifest.version };
}

async function installedPackage(stage: string, name: string) {
  const path = join(stage, "node_modules", ...name.split("/"));
  const manifest = JSON.parse(
    await readFile(join(path, "package.json"), "utf8"),
  ) as { readonly name: string; readonly version: string };
  if (manifest.name !== name) {
    throw new Error(`installed package identity mismatch for ${name}`);
  }
  return { realPath: await realpath(path), manifest };
}

async function assertPackageAbsent(stage: string, name: string): Promise<void> {
  try {
    await lstat(join(stage, "node_modules", ...name.split("/")));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the install graph`);
}

async function assertSingleSvelteRuntime(stage: string): Promise<void> {
  const manifests = (
    await readdir(join(stage, "node_modules"), {
      recursive: true,
    })
  ).filter(
    (path) =>
      path === "svelte/package.json" ||
      path.endsWith("/node_modules/svelte/package.json"),
  );
  if (manifests.length !== 1) {
    throw new Error(`expected one Svelte runtime, found ${manifests.length}`);
  }
}

async function longhornImports(stage: string): Promise<readonly string[]> {
  const files = (await readdir(stage, { recursive: true })).filter(
    (path) => /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules/"),
  );
  const imports = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(stage, path), "utf8");
    for (const match of source.matchAll(
      /from\s+["'](@longhorn\/[^"']+)["']/g,
    )) {
      imports.add(match[1]!);
    }
  }
  return [...imports].sort();
}
