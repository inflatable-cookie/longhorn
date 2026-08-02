import { createHash, randomUUID } from "node:crypto";
import {
  copyFile,
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

type ConsumerName = "soundcheck" | "bovine" | "jetstream";

type ConsumerSelection = {
  selected_typescript: string[];
  selected_rust: string[];
  forbidden_systems: string[];
};

type FreezeFixture = {
  sources: {
    longhorn_compilation_commit: string;
    soundcheck_commit: string;
    soundcheck_library_commit: string;
    signal_commit: string;
    bovine_commit: string;
    jetstream_commit: string;
    poodle_commit: string;
    poodle_prior_artifact_set: string;
  };
  consumers: Record<ConsumerName, ConsumerSelection>;
  publication: {
    package_managers: boolean;
    tags: boolean;
    hosted_releases: boolean;
  };
};

type AdmissionFixture = {
  schema: string;
  outcome: string;
  sources: {
    longhorn_selected_source_commit: string;
    longhorn_selected_tree_sha256: string;
    poodle_commit: string;
  };
  artifact_sets: {
    poodle: string;
    longhorn_typescript: string;
    longhorn_rust: string;
    rust_lock: string;
  };
  tools: {
    rust: string;
    tauri_rust: string;
    bun: string;
    typescript: string;
    svelte: string;
    tauri_api: string;
  };
  artifacts: {
    poodle: number;
    longhorn_typescript: number;
    longhorn_rust: number;
    rust_workspace_support: number;
  };
  graphs: Record<ConsumerName, { typescript: number; rust: number }>;
  audits: {
    generated_bindings_drift: boolean;
    original_workspace_resolution: boolean;
    sibling_source_resolution: boolean;
    one_svelte_runtime_per_renderer: boolean;
    independent_native_content_mechanisms: boolean;
    consumer_repository_writes: boolean;
    package_manager_publication: boolean;
    tags: boolean;
    hosted_releases: boolean;
  };
  write_admission: { next_card: number; scope: string };
};

type ArtifactIdentity = {
  name: string;
  version: string;
  filename: string;
  sha256: string;
};

type PackageManifest = {
  name: string;
  version: string;
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  peerDependenciesMeta?: Record<string, { optional?: boolean }>;
};

const repoRoot = resolve(import.meta.dir, "..");
const poodleRoot = resolve(
  process.env.POODLE_REPO ?? resolve(repoRoot, "../poodle"),
);
const fixture = JSON.parse(
  await readFile(
    resolve(
      repoRoot,
      "fixtures/migration/secondary-consumer-card113/behavior-authority-freeze-v1.json",
    ),
    "utf8",
  ),
) as FreezeFixture;
const admission = JSON.parse(
  await readFile(
    resolve(
      repoRoot,
      "fixtures/migration/secondary-consumer-card114/private-artifact-admission-v1.json",
    ),
    "utf8",
  ),
) as AdmissionFixture;

const consumerRepositories = {
  soundcheck: resolve(
    process.env.SOUNDCHECK_REPO ?? resolve(repoRoot, "../soundcheck"),
  ),
  soundcheck_library: resolve(
    process.env.SOUNDCHECK_LIBRARY_REPO ??
      resolve(repoRoot, "../soundcheck-library"),
  ),
  signal: resolve(process.env.SIGNAL_REPO ?? resolve(repoRoot, "../signal")),
  bovine: resolve(
    process.env.BOVINE_REPO ??
      resolve(repoRoot, "../acowtancy/bovine-accelerator-desktop"),
  ),
  jetstream: resolve(
    process.env.JETSTREAM_REPO ?? resolve(repoRoot, "../jetstream"),
  ),
} as const;

const packageDirectories = new Map(
  [
    "core",
    "config",
    "settings",
    "operation",
    "native-content",
    "native-content-svelte",
    "tauri",
    "bridge",
    "commands",
  ].map((directory) => [`@longhorn/${directory}`, directory]),
);
const poodlePackages = [
  ["@poodle/headless", "packages/core"],
  ["@poodle/styles", "packages/styles"],
  ["@poodle/svelte-tokens", "packages/svelte/tokens"],
  ["@poodle/icons-lucide", "packages/svelte/icons-lucide"],
  ["@poodle/svelte", "packages/svelte/components"],
] as const;
const rustSupportCrates = [
  "longhorn-layout",
  "longhorn-surfaces",
  "longhorn-surface-windowing",
] as const;
const generatedDomains = [
  "config",
  "settings",
  "operation",
  "native-content",
  "bridge",
  "commands",
] as const;

const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-secondary-consumer-private-artifacts-"),
);

try {
  const sources = await verifySources();
  await verifyGeneratedBindings();
  const poodle = await packPoodleArtifacts();
  const typescript = await packLonghornTypescriptArtifacts();
  const rendererGraphs = {} as Record<ConsumerName, unknown>;
  for (const name of consumerNames()) {
    rendererGraphs[name] = await verifyRendererGraph(
      name,
      typescript.paths,
      poodle,
    );
  }
  const rust = await packAndVerifyRustGraphs();
  verifyAdmissionEvidence(poodle, typescript, rust);

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.secondary-consumer-private-artifact-proof.v1",
        outcome: "pass",
        sources,
        tools: {
          rust: "1.85.0",
          tauriRust: "2.11.5",
          bun: (await run(["bun", "--version"], repoRoot)).trim(),
          typescript: "6.0.3",
          svelte: "5.56.8",
          tauriApi: "2.11.1",
        },
        artifacts: {
          poodleArtifactSet: poodle.artifactSet,
          poodle: poodle.identities,
          longhornTypescriptArtifactSet: typescript.artifactSet,
          longhornTypescript: typescript.identities,
          longhornRustArtifactSet: rust.artifactSet,
          longhornRust: rust.identities,
          rustWorkspaceSupport: rust.supportIdentities,
        },
        rendererGraphs,
        rustGraphs: rust.graphs,
        developmentMaps: privateDevelopmentMaps(),
        rollback: {
          card113ReceiptsRetained: true,
          consumerLocksRetained: true,
          donorSourcesRetained: true,
        },
        audits: {
          generatedBindingsDrift: false,
          isolatedProducedArtifactInstalls: true,
          originalWorkspaceResolutionInProof: false,
          siblingSourceResolutionInProof: false,
          oneSvelteRuntimePerRenderer: true,
          exactPoodleArtifactSet: true,
          independentNativeContentMechanisms: true,
          consumerRepositoriesWritten: false,
          packageManagerPublication: fixture.publication.package_managers,
          tags: fixture.publication.tags,
          hostedReleases: fixture.publication.hosted_releases,
        },
        writeAdmission: {
          admitted: true,
          nextCard: 115,
          scope: "Soundcheck storage, config, and protected primary window",
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_SECONDARY_CONSUMER_ARTIFACT_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function verifySources() {
  const proofHead = (await run(["git", "rev-parse", "HEAD"], repoRoot)).trim();
  const poodleCommit = (
    await run(["git", "rev-parse", "HEAD"], poodleRoot)
  ).trim();
  assertEqual(
    poodleCommit,
    fixture.sources.poodle_commit,
    "Poodle source commit",
  );
  await assertClean("Poodle", poodleRoot);
  await assertSelectedLonghornSourcesClean();
  await run(
    [
      "git",
      "cat-file",
      "-e",
      `${admission.sources.longhorn_selected_source_commit}^{commit}`,
    ],
    repoRoot,
  );
  assertEqual(
    await selectedLonghornTreeHash(
      admission.sources.longhorn_selected_source_commit,
    ),
    admission.sources.longhorn_selected_tree_sha256,
    "selected Longhorn tree",
  );

  const expected = {
    soundcheck: fixture.sources.soundcheck_commit,
    soundcheck_library: fixture.sources.soundcheck_library_commit,
    signal: fixture.sources.signal_commit,
    bovine: fixture.sources.bovine_commit,
    jetstream: fixture.sources.jetstream_commit,
  } as const;
  for (const [name, root] of Object.entries(consumerRepositories)) {
    assertEqual(
      (await run(["git", "rev-parse", "HEAD"], root)).trim(),
      expected[name as keyof typeof expected],
      `${name} retained source`,
    );
  }

  return {
    longhorn: {
      selectedSourceCommit: admission.sources.longhorn_selected_source_commit,
      proofHead,
      selectedTreeSha256: admission.sources.longhorn_selected_tree_sha256,
      selectedSourcesClean: true,
    },
    poodle: { commit: poodleCommit, clean: true },
    consumers: expected,
  };
}

async function assertSelectedLonghornSourcesClean() {
  const paths = selectedLonghornPaths();
  const status = await run(
    ["git", "status", "--porcelain", "--", ...paths],
    repoRoot,
  );
  assert(!status.trim(), `selected Longhorn sources are dirty:\n${status}`);
  const changed = await run(
    [
      "git",
      "diff",
      "--name-only",
      `${admission.sources.longhorn_selected_source_commit}..HEAD`,
      "--",
      ...paths,
    ],
    repoRoot,
  );
  assert(
    !changed.trim(),
    `selected Longhorn sources changed after admission receipt:\n${changed}`,
  );
}

function selectedLonghornPaths() {
  return [
    ...[...packageDirectories.values()].map(
      (directory) => `packages/${directory}`,
    ),
    ...selectedRustCrates().map((name) => `crates/${name}`),
    ...rustSupportCrates.map((name) => `crates/${name}`),
    "Cargo.toml",
    "Cargo.lock",
  ];
}

async function selectedLonghornTreeHash(commit: string) {
  return hash(
    await run(
      ["git", "ls-tree", "-r", commit, "--", ...selectedLonghornPaths()],
      repoRoot,
    ),
  );
}

async function verifyGeneratedBindings() {
  for (const domain of generatedDomains) {
    await run(
      ["cargo", "run", "-p", "longhorn-bindings", "--", domain, "check"],
      repoRoot,
    );
  }
}

async function packPoodleArtifacts() {
  const artifactRoot = join(temporaryRoot, "poodle-artifacts");
  await mkdir(artifactRoot);
  const identities: ArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of poodlePackages) {
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
      join(poodleRoot, directory),
    );
    const path = join(
      artifactRoot,
      `${name.replace("@", "").replace("/", "-")}-0.1.0.tgz`,
    );
    const identity = await inspectTypescriptArtifact(name, path);
    identities.push(identity);
    paths.set(name, path);
  }
  const artifactSet = artifactSetId(identities);
  assertEqual(
    artifactSet,
    fixture.sources.poodle_prior_artifact_set,
    "fresh Poodle artifact set",
  );
  return { paths, identities, artifactSet };
}

async function packLonghornTypescriptArtifacts() {
  const artifactRoot = join(temporaryRoot, "longhorn-typescript-artifacts");
  await mkdir(artifactRoot);
  const identities: ArtifactIdentity[] = [];
  const paths = new Map<string, string>();
  for (const [name, directory] of packageDirectories) {
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
    const identity = await inspectTypescriptArtifact(name, path);
    identities.push(identity);
    paths.set(name, path);
  }
  return { paths, identities, artifactSet: artifactSetId(identities) };
}

async function inspectTypescriptArtifact(name: string, path: string) {
  const listing = await run(["tar", "-tzf", path], temporaryRoot);
  assert(!listing.includes("node_modules/"), `${name} packs node_modules`);
  assert(!listing.includes("/tests/"), `${name} packs tests`);
  const manifest = JSON.parse(
    await run(["tar", "-xOzf", path, "package/package.json"], temporaryRoot),
  ) as PackageManifest;
  assertEqual(manifest.name, name, `${name} packed name`);
  assertEqual(manifest.version, "0.1.0", `${name} packed version`);
  const serialized = JSON.stringify(manifest);
  for (const alias of ["workspace:", "link:"]) {
    assert(!serialized.includes(alias), `${name} manifest contains ${alias}`);
  }
  return {
    name,
    version: manifest.version,
    filename: basename(path),
    sha256: await digest(path),
  };
}

async function verifyRendererGraph(
  consumer: ConsumerName,
  longhornArtifacts: ReadonlyMap<string, string>,
  poodle: {
    paths: ReadonlyMap<string, string>;
    identities: ArtifactIdentity[];
  },
) {
  const stage = join(temporaryRoot, `renderer-${consumer}-${randomUUID()}`);
  await mkdir(join(stage, "src"), { recursive: true });
  const selected = fixture.consumers[consumer].selected_typescript;
  const dependencies: Record<string, string> = {
    "@tauri-apps/api": "2.11.1",
    svelte: "5.56.8",
  };
  for (const name of selected) {
    const path = longhornArtifacts.get(name);
    assert(path, `${consumer} has no artifact for ${name}`);
    dependencies[name] = fileDependency(path);
  }
  for (const [name, path] of poodle.paths) {
    dependencies[name] = fileDependency(path);
  }
  await writeFile(
    join(stage, "package.json"),
    `${JSON.stringify(
      {
        name: `longhorn-${consumer}-private-artifact-proof`,
        private: true,
        type: "module",
        dependencies,
        devDependencies: {
          "@types/bun": "1.3.14",
          "svelte-check": "4.7.4",
          typescript: "6.0.3",
        },
        overrides: dependencies,
      },
      null,
      2,
    )}\n`,
  );
  await writeFile(join(stage, "tsconfig.json"), rendererTsconfig());
  await writeFile(
    join(stage, "src", "artifact-contract.ts"),
    rendererContract(selected),
  );
  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "./tsconfig.json"], stage);
  await run(
    ["bun", "x", "svelte-check", "--tsconfig", "./tsconfig.json"],
    stage,
  );

  const installed = [];
  for (const name of [...selected, ...poodle.paths.keys()]) {
    installed.push(await assertArtifactInstall(stage, name));
  }
  for (const forbidden of forbiddenTypescriptPackages(consumer)) {
    await assertPackageAbsent(stage, forbidden);
  }
  await assertSingleSvelteRuntime(stage);
  const lock = await readFile(join(stage, "bun.lock"), "utf8");
  for (const alias of [
    resolve(repoRoot, "packages"),
    resolve(poodleRoot, "packages"),
    "workspace:",
    "link:",
  ]) {
    assert(
      !lock.includes(alias),
      `${consumer} lock contains source alias ${alias}`,
    );
  }
  return {
    installed,
    selected,
    poodleArtifactSet: artifactSetId(poodle.identities),
    svelte: "5.56.8",
    tauriApi: "2.11.1",
    typescript: "6.0.3",
    oneSvelteRuntime: true,
    sourceAliases: false,
    forbiddenPackagesAbsent: forbiddenTypescriptPackages(consumer),
    lockSha256: await digest(join(stage, "bun.lock")),
  };
}

function rendererTsconfig() {
  return `${JSON.stringify(
    {
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
    },
    null,
    2,
  )}\n`;
}

function rendererContract(selected: string[]) {
  const imports = selected
    .map(
      (name, index) =>
        `import * as package${index} from ${JSON.stringify(name)};`,
    )
    .join("\n");
  const members = selected.map((_, index) => `package${index}`).join(", ");
  return `${imports}\n\nexport const exactPrivateGraph = [${members}] as const;\n`;
}

async function assertArtifactInstall(stage: string, name: string) {
  const root = installedPackageRoot(stage, name);
  const resolved = await realpath(root);
  for (const sourceRoot of [
    resolve(repoRoot, "packages"),
    resolve(poodleRoot, "packages"),
  ]) {
    assert(
      !resolved.startsWith(`${sourceRoot}/`),
      `${name} resolved to sibling source ${resolved}`,
    );
  }
  const manifest = JSON.parse(
    await readFile(join(root, "package.json"), "utf8"),
  ) as PackageManifest;
  assertEqual(manifest.version, "0.1.0", `${name} installed version`);
  return { name, version: manifest.version };
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

async function assertSingleSvelteRuntime(stage: string) {
  const manifests = (
    await readdir(join(stage, "node_modules"), {
      recursive: true,
    })
  ).filter(
    (path) =>
      path === "svelte/package.json" ||
      path.endsWith("/node_modules/svelte/package.json"),
  );
  assertEqual(manifests.length, 1, "Svelte runtime count");
}

function installedPackageRoot(stage: string, name: string) {
  return join(stage, "node_modules", ...name.split("/"));
}

async function packAndVerifyRustGraphs() {
  const artifactRoot = join(temporaryRoot, "rust-artifacts");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(artifactRoot);
  await mkdir(workspace);
  const selected = selectedRustCrates();
  const identities: ArtifactIdentity[] = [];
  const supportIdentities: ArtifactIdentity[] = [];
  for (const name of [...selected, ...rustSupportCrates]) {
    const inventory = await run(
      ["cargo", "package", "-p", name, "--list", "--allow-dirty"],
      repoRoot,
    );
    assert(
      inventory.includes("Cargo.toml"),
      `${name} inventory lacks Cargo.toml`,
    );
    assert(
      inventory.includes("src/lib.rs"),
      `${name} inventory lacks src/lib.rs`,
    );
    const archive = join(artifactRoot, `${name}-0.1.0.private.tar.gz`);
    await run(
      [
        "git",
        "archive",
        "--format=tar.gz",
        `--output=${archive}`,
        admission.sources.longhorn_selected_source_commit,
        `crates/${name}`,
      ],
      repoRoot,
    );
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    const identity = {
      name,
      version: "0.1.0",
      filename: basename(archive),
      sha256: await digest(archive),
    };
    if ((rustSupportCrates as readonly string[]).includes(name)) {
      supportIdentities.push(identity);
    } else {
      identities.push(identity);
    }
  }

  await mkdir(join(workspace, "consumers"), { recursive: true });
  await writeFile(
    join(workspace, "Cargo.toml"),
    rustWorkspaceManifest(selected),
  );
  for (const consumer of consumerNames()) {
    await writeRustConsumer(workspace, consumer);
  }
  await copyFile(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));
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

  const graphs = {} as Record<ConsumerName, unknown>;
  for (const consumer of consumerNames()) {
    graphs[consumer] = await verifyRustGraph(workspace, consumer);
  }
  return {
    identities,
    supportIdentities,
    artifactSet: artifactSetId(identities),
    graphs,
  };
}

async function writeRustConsumer(workspace: string, consumer: ConsumerName) {
  const root = join(workspace, "consumers", consumer);
  await mkdir(join(root, "src"), { recursive: true });
  const dependencies = fixture.consumers[consumer].selected_rust
    .map((name) => `${name}.workspace = true`)
    .join("\n");
  await writeFile(
    join(root, "Cargo.toml"),
    `[package]\nname = "longhorn-${consumer}-private-artifact-proof"\nversion = "0.0.0"\nedition.workspace = true\n\n[dependencies]\n${dependencies}\n`,
  );
  await writeFile(join(root, "src", "main.rs"), "fn main() {}\n");
}

async function verifyRustGraph(workspace: string, consumer: ConsumerName) {
  const packageName = `longhorn-${consumer}-private-artifact-proof`;
  await run(
    ["cargo", "+1.85.0", "check", "-p", packageName, "--locked", "--offline"],
    workspace,
  );
  const tree = await run(
    [
      "cargo",
      "+1.85.0",
      "tree",
      "-p",
      packageName,
      "--locked",
      "--offline",
      "--edges",
      "normal",
      "--prefix",
      "none",
    ],
    workspace,
  );
  for (const required of fixture.consumers[consumer].selected_rust) {
    assert(
      tree.includes(`${required} v0.1.0`),
      `${consumer} Rust graph lacks ${required}`,
    );
  }
  const forbidden = forbiddenRustPackages(consumer);
  for (const name of forbidden) {
    assert(
      !tree.includes(`${name} v`),
      `${consumer} Rust graph contains ${name}`,
    );
  }
  assert(
    tree.includes("tauri v2.11.5"),
    `${consumer} did not resolve Tauri 2.11.5`,
  );
  return {
    selected: fixture.consumers[consumer].selected_rust,
    forbiddenPackagesAbsent: forbidden,
    rust: "1.85.0",
    tauri: "2.11.5",
    locked: true,
    offline: true,
    cargoLockSha256: await digest(join(workspace, "Cargo.lock")),
  };
}

function rustWorkspaceManifest(selected: string[]) {
  const members = [
    ...selected.map((name) => `crates/${name}`),
    ...rustSupportCrates.map((name) => `crates/${name}`),
    ...consumerNames().map((name) => `consumers/${name}`),
  ];
  const internal = [...selected, ...rustSupportCrates]
    .map((name) => `${name} = { path = "crates/${name}", version = "0.1.0" }`)
    .join("\n");
  return `[workspace]\nmembers = ${JSON.stringify(members, null, 2)}\nresolver = "2"\n\n[workspace.package]\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.85"\nlicense = "MIT"\nrepository = "https://github.com/inflatable-cookie/longhorn"\n\n[workspace.dependencies]\n${internal}\ncap-std = "4.0.2"\nfs4 = { version = "1.1.0", default-features = false, features = ["sync"] }\nproptest = { version = "1.11.0", default-features = false, features = ["std"] }\nrusqlite = { version = "=0.31.0", features = ["backup", "bundled"] }\nserde = { version = "1.0.229", features = ["derive"] }\nserde_json = "1.0.151"\nsha2 = "0.10.9"\ntauri = { version = "=2.11.5", default-features = false }\ntempfile = "3.27.0"\nts-rs = { version = "=11.0.0", default-features = false, features = ["no-serde-warnings", "serde-compat"] }\nzip = { version = "=5.1.1", default-features = false, features = ["deflate-flate2-zlib-rs"] }\n\n[workspace.lints.rust]\nmissing_docs = "deny"\nunsafe_code = "forbid"\n\n[workspace.lints.clippy]\nall = "deny"\n`;
}

function selectedRustCrates() {
  return [
    ...new Set(
      consumerNames().flatMap(
        (consumer) => fixture.consumers[consumer].selected_rust,
      ),
    ),
  ].sort();
}

function verifyAdmissionEvidence(
  poodle: { identities: ArtifactIdentity[]; artifactSet: string },
  typescript: { identities: ArtifactIdentity[]; artifactSet: string },
  rust: {
    identities: ArtifactIdentity[];
    supportIdentities: ArtifactIdentity[];
    artifactSet: string;
    graphs: Record<ConsumerName, unknown>;
  },
) {
  assertEqual(
    admission.schema,
    "longhorn.secondary-consumer-private-artifact-admission.v1",
    "admission schema",
  );
  assertEqual(admission.outcome, "pass", "admission outcome");
  assertEqual(admission.tools.rust, "1.85.0", "Rust toolchain");
  assertEqual(admission.tools.tauri_rust, "2.11.5", "Rust Tauri version");
  assertEqual(admission.tools.bun, "1.3.14", "Bun version");
  assertEqual(admission.tools.typescript, "6.0.3", "TypeScript version");
  assertEqual(admission.tools.svelte, "5.56.8", "Svelte version");
  assertEqual(admission.tools.tauri_api, "2.11.1", "Tauri API version");
  assertEqual(
    admission.sources.poodle_commit,
    fixture.sources.poodle_commit,
    "admitted Poodle commit",
  );
  assertEqual(poodle.artifactSet, admission.artifact_sets.poodle, "Poodle set");
  assertEqual(
    typescript.artifactSet,
    admission.artifact_sets.longhorn_typescript,
    "Longhorn TypeScript set",
  );
  assertEqual(
    rust.artifactSet,
    admission.artifact_sets.longhorn_rust,
    "Longhorn Rust set",
  );
  assertEqual(
    poodle.identities.length,
    admission.artifacts.poodle,
    "Poodle artifacts",
  );
  assertEqual(
    typescript.identities.length,
    admission.artifacts.longhorn_typescript,
    "Longhorn TypeScript artifacts",
  );
  assertEqual(
    rust.identities.length,
    admission.artifacts.longhorn_rust,
    "Longhorn Rust artifacts",
  );
  assertEqual(
    rust.supportIdentities.length,
    admission.artifacts.rust_workspace_support,
    "Rust support artifacts",
  );
  for (const consumer of consumerNames()) {
    assertEqual(
      fixture.consumers[consumer].selected_typescript.length,
      admission.graphs[consumer].typescript,
      `${consumer} TypeScript graph size`,
    );
    assertEqual(
      fixture.consumers[consumer].selected_rust.length,
      admission.graphs[consumer].rust,
      `${consumer} Rust graph size`,
    );
    const graph = rust.graphs[consumer] as { cargoLockSha256: string };
    assertEqual(
      graph.cargoLockSha256,
      admission.artifact_sets.rust_lock,
      `${consumer} Rust lock`,
    );
  }
  assertEqual(
    admission.audits.generated_bindings_drift,
    false,
    "binding drift",
  );
  assertEqual(
    admission.audits.original_workspace_resolution,
    false,
    "original workspace resolution",
  );
  assertEqual(
    admission.audits.sibling_source_resolution,
    false,
    "sibling source resolution",
  );
  assertEqual(
    admission.audits.one_svelte_runtime_per_renderer,
    true,
    "single Svelte runtime",
  );
  assertEqual(
    admission.audits.independent_native_content_mechanisms,
    true,
    "independent native-content mechanisms",
  );
  assertEqual(
    admission.audits.consumer_repository_writes,
    false,
    "consumer writes",
  );
  assertEqual(
    admission.audits.package_manager_publication,
    false,
    "package publication",
  );
  assertEqual(admission.audits.tags, false, "tags");
  assertEqual(admission.audits.hosted_releases, false, "hosted releases");
  assertEqual(admission.write_admission.next_card, 115, "next admitted card");
  assertEqual(
    admission.write_admission.scope,
    "soundcheck-storage-config-protected-primary-window",
    "write admission scope",
  );
}

function forbiddenTypescriptPackages(consumer: ConsumerName) {
  const selected = new Set(fixture.consumers[consumer].selected_typescript);
  return [...packageDirectories.keys()].filter((name) => !selected.has(name));
}

function forbiddenRustPackages(consumer: ConsumerName) {
  const selected = new Set(fixture.consumers[consumer].selected_rust);
  return selectedRustCrates().filter((name) => !selected.has(name));
}

function privateDevelopmentMaps() {
  return Object.fromEntries(
    consumerNames().map((consumer) => [
      consumer,
      {
        typescript: Object.fromEntries(
          fixture.consumers[consumer].selected_typescript.map((name) => [
            name,
            `file:../../longhorn/packages/${packageDirectories.get(name)}`,
          ]),
        ),
        rust: Object.fromEntries(
          fixture.consumers[consumer].selected_rust.map((name) => [
            name,
            { path: `../../longhorn/crates/${name}` },
          ]),
        ),
        evidenceRule:
          "source links require exact clean commits; compatibility comes from produced artifacts",
      },
    ]),
  );
}

function consumerNames(): ConsumerName[] {
  return ["soundcheck", "bovine", "jetstream"];
}

function artifactSetId(artifacts: ArtifactIdentity[]) {
  return hash(
    artifacts.map(({ name, sha256 }) => `${name}:${sha256}`).join("\n"),
  );
}

function fileDependency(path: string) {
  return `file:${path}`;
}

async function assertClean(label: string, root: string) {
  const status = await run(["git", "status", "--porcelain"], root);
  assert(!status.trim(), `${label} worktree is dirty:\n${status}`);
}

async function digest(path: string) {
  return hash(await readFile(path));
}

function hash(value: string | Uint8Array) {
  return createHash("sha256").update(value).digest("hex");
}

async function run(command: string[], cwd: string): Promise<string> {
  const child = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`,
    );
  }
  return `${stdout}\n${stderr}`;
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string) {
  if (actual !== expected) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}
