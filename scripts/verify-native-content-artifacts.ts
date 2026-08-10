import { poodleRelease } from "./poodle-release.ts";
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

// Poodle installs from the registry; poodleRelease() checks each published
// package's sha512 against bun.lock and against the installed copy.
const POODLE_RELEASE = poodleRelease();
const NATIVE_CONTENT_PROTOCOL_FIXTURE =
  "bdfed54fc5f9c70d82485c5b572e3b2be3663c7bc37c6f565a8d4fc48196557e";
// Rebaselined 2026-08-10 for Card 160. The fixture gained a `hostDestroy`
// category because no category reached `HostDestroyReceipt`, so its field list
// was enforcing nothing. The generated TypeScript digest is unchanged, and the
// navigation-absence check below is the substantive guard here — it still
// passes over the new payload.
// Rebaselined 2026-08-08 for Card 164. The generated file changed only in its
// import header — `@inflatable-cookie/longhorn-core` became
// `@inflatable-cookie/longhorn/core` — and `check:bindings` confirms it still
// matches the Rust authority byte for byte. The constant exists to catch
// unintended drift, so it is updated deliberately rather than relaxed.
const NATIVE_CONTENT_TYPESCRIPT_PROTOCOL =
  "1770327218ac9381fc59613d966040d550331c57eb25895d8c7e03dd4c06cd08";
const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "native-content-system-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-native-content-artifact-proof-"),
);
const typescriptArtifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(typescriptArtifactRoot);

type Shape = "nucleus" | "soundcheck" | "jetstream";

interface ArtifactIdentity {
  readonly name: string;
  readonly filename: string;
  readonly sha256: string;
}

interface PackageManifest {
  readonly name: string;
  dependencies: Record<string, string>;
  devDependencies?: Record<string, string>;
  overrides?: Record<string, string>;
}

try {
  await run(
    ["cargo", "run", "-p", "longhorn-bindings", "--", "native-content", "check"],
    repoRoot,
  );
  const protocolIsolation = await verifyProtocolIsolation();
  const typescript = await packTypescriptArtifacts();
  const rust = await packAndRunRustArtifacts();
  const consumers = await Promise.all(
    (["nucleus", "soundcheck", "jetstream"] as const).map((shape) =>
      verifyConsumer(shape, typescript.paths, rust.traces[shape]),
    ),
  );
  const packagedMechanisms = await verifyPackagedMechanisms();
  const prototypes = await verifyPrototypeDisposition();

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.native-content-artifact-proof.v1",
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustGraphs: rust.graphs,
        nativeTraces: rust.traces,
        typescriptArtifacts: typescript.identities,
        consumers,
        packagedMechanisms,
        prototypes,
        migrationPrerequisites: {
          nucleus: {
            requiredArtifacts: [
              "longhorn-native-content",
              "longhorn-tauri-native-content-child-view",
              "@inflatable-cookie/longhorn-native-content",
              "@inflatable-cookie/longhorn-native-content-svelte",
            ],
            artifactGate: "pass",
            nextAdmission: "migration_planning",
            nativeCutover: "blocked_on_browser_policy_map",
            consumerAuthority: [
              "browser construction and source URL",
              "navigation, popup, download, permission, and data-store policy",
              "panel activity, overlay intersection, and final visibility policy",
              "remote child capability policy",
            ],
          },
          soundcheck: {
            requiredArtifacts: [
              "longhorn-native-content",
              "longhorn-native-content-isolated-window",
            ],
            artifactGate: "pass",
            nextAdmission: "after_g01_014_and_g01_015",
            nativeCutover: "blocked_on_dependency_runway_and_consumer_mapping",
            consumerAuthority: [
              "Signal/plugin ABI and authorization",
              "helper launch, process isolation, and owner termination",
              "audio, MIDI, presets, screenshots, and native root-view policy",
            ],
          },
          jetstream: {
            requiredArtifacts: [
              "longhorn-native-content",
              "longhorn-native-content-backing-surface",
              "@inflatable-cookie/longhorn-native-content",
              "@inflatable-cookie/longhorn-native-content-svelte",
            ],
            artifactGate: "pass",
            nextAdmission: "after_g01_014_and_g01_015",
            nativeCutover: "blocked_on_dependency_runway_and_consumer_mapping",
            consumerAuthority: [
              "native storage and WGPU surface construction",
              "renderer, world, camera, picking, gizmo, and frame loop",
              "semantic pointer, button, wheel, and keyboard mapping",
            ],
          },
          loophole: {
            requiredArtifacts: [],
            artifactGate: "not_applicable",
            nextAdmission: "no_current_native_content_migration",
          },
        },
        audits: {
          generatedBindingsDrift: false,
          childNavigationAbsentFromCommonProtocol: protocolIsolation.navigationAbsent,
          commonProtocolDigests: protocolIsolation.digests,
          producedArtifactGraphsIsolated: true,
          rustRendererThreeShapeParity: true,
          selectedSvelteGraphsCompile: true,
          publicPoodleOnly: true,
          optionalMechanismsAbsentWhenUnselected: true,
          browserPluginGpuPayloadsAbsent: true,
          rawHandleAndOuterPlacementAuthorityExternal: true,
          capabilitiesProtocolOnly: true,
          liveScaleClaimsBoundedToEvidence: true,
          supportStatusesExact: true,
          prototypesNonPublishable: true,
          donorRepositoriesWritten: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_NATIVE_CONTENT_ARTIFACT_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function verifyProtocolIsolation() {
  const fixturePath = join(repoRoot, "fixtures", "native-content", "protocol-v1.json");
  const typescriptPath = join(
    repoRoot,
    "packages",
    "longhorn",
    "src",
    "native-content",
    "generated",
    "protocol.ts",
  );
  const digests = {
    fixture: await digest(fixturePath),
    typescript: await digest(typescriptPath),
  };
  if (digests.fixture !== NATIVE_CONTENT_PROTOCOL_FIXTURE) {
    throw new Error("native-content protocol fixture changed during child navigation work");
  }
  if (digests.typescript !== NATIVE_CONTENT_TYPESCRIPT_PROTOCOL) {
    throw new Error("native-content generated TypeScript changed during child navigation work");
  }
  const commonProtocol = `${await readFile(fixturePath, "utf8")}\n${await readFile(typescriptPath, "utf8")}`;
  if (/navigate|current_url|requested_url/i.test(commonProtocol)) {
    throw new Error("child navigation entered the common renderer protocol");
  }
  return { navigationAbsent: true, digests };
}

async function packTypescriptArtifacts() {
  const packages = [["@inflatable-cookie/longhorn", "longhorn"], ["@inflatable-cookie/longhorn-poodle-svelte", "longhorn-poodle-svelte"]] as const;
  const identities: ArtifactIdentity[] = [];
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
    await inspectTypescriptArtifact(name, path);
    paths.set(name, path);
    identities.push({ name, filename: basename(path), sha256: await digest(path) });
  }
  return { identities, paths };
}

async function inspectTypescriptArtifact(name: string, path: string) {
  const listing = await run(["tar", "-tzf", path], typescriptArtifactRoot);
  if (
    listing.includes("node_modules/") ||
    listing.includes("/tests/") ||
    listing.includes("workspace:")
  ) {
    throw new Error(`${name} artifact contains proof or workspace material`);
  }
  const manifest = JSON.parse(
    await run(["tar", "-xOzf", path, "package/package.json"], typescriptArtifactRoot),
  ) as {
    readonly name: string;
    readonly version: string;
    readonly dependencies?: Record<string, string>;
    readonly peerDependencies?: Record<string, string>;
    readonly exports?: Record<string, unknown>;
  };
  if (manifest.name !== name || manifest.version !== "0.1.0") {
    throw new Error(`${name} packed identity mismatch`);
  }
  // Card 164: both tiers carry no dependencies at all. The framework package
  // has none by construction, and the projection tier expresses its needs as
  // peers rather than dependencies.
  const expectedDependencies: Record<string, readonly string[]> = {
    "@inflatable-cookie/longhorn": [],
    "@inflatable-cookie/longhorn-poodle-svelte": [],
  };
  assertExactSet(
    `${name} dependencies`,
    Object.keys(manifest.dependencies ?? {}),
    expectedDependencies[name] ?? [],
  );
  if (name === "@inflatable-cookie/longhorn-native-content-svelte") {
    assertExactSet(
      "native-content Svelte peers",
      Object.keys(manifest.peerDependencies ?? {}),
      ["svelte"],
    );
    if (manifest.peerDependencies?.svelte !== ">=5.38.6 <6") {
      throw new Error("native-content Svelte peer range changed");
    }
  }
  if (name === "@inflatable-cookie/longhorn-native-content") {
    assertExactSet(
      "native-content exports",
      Object.keys(manifest.exports ?? {}),
      [".", "./package.json", "./protocol", "./tauri"],
    );
  }

  const extractRoot = join(
    typescriptArtifactRoot,
    `inspect-${name.replace("@", "").replace("/", "-")}`,
  );
  await mkdir(extractRoot);
  await run(["tar", "-xzf", path, "-C", extractRoot], typescriptArtifactRoot);
  // Card 164: the projection tier is one package, so scanning the whole
  // artifact for a Poodle edge would flag every unrelated Poodle projection in
  // it. The claim was always about the native-content tier specifically, so it
  // is scoped to that subtree; the mechanism markers still apply everywhere.
  const packageSrc = join(extractRoot, "package", "src");
  const source = await readSourceTree(packageSrc);
  for (const marker of ["SignalPlugin", "wgpu", "devicePixelRatio", "querySelector"]) {
    if (source.includes(marker)) {
      throw new Error(`${name} artifact contains forbidden authority marker ${marker}`);
    }
  }
  const nativeContentSource =
    name === "@inflatable-cookie/longhorn-poodle-svelte"
      ? await readSourceTree(join(packageSrc, "native-content"))
      : source;
  if (nativeContentSource.includes("@inflatable-cookie/poodle-")) {
    throw new Error(
      `${name} native-content tier contains a Poodle edge`,
    );
  }
}

async function packAndRunRustArtifacts() {
  const crates = [
    "longhorn-core",
    "longhorn-native-content",
    "longhorn-tauri-native-content-child-view",
    "longhorn-native-content-isolated-window",
    "longhorn-native-content-backing-surface",
  ] as const;
  const archiveRoot = join(temporaryRoot, "rust-archives");
  const workspace = join(temporaryRoot, "rust-workspace");
  await mkdir(archiveRoot);
  await mkdir(workspace);
  const identities: ArtifactIdentity[] = [];
  for (const name of crates) {
    const inventory = await run(
      ["cargo", "+1.95.0", "package", "-p", name, "--list", "--allow-dirty"],
      repoRoot,
    );
    if (!inventory.includes("Cargo.toml") || !inventory.includes("src/lib.rs")) {
      throw new Error(`${name} source inventory is incomplete`);
    }
    const archive = join(archiveRoot, `${name}-0.1.0.private.tar.gz`);
    await run(["tar", "-czf", archive, "-C", repoRoot, `crates/${name}`], repoRoot);
    await run(["tar", "-xzf", archive, "-C", workspace], repoRoot);
    identities.push({ name, filename: basename(archive), sha256: await digest(archive) });
  }

  const sharedSource = (
    await Promise.all(
      crates.map((name) => readSourceTree(join(workspace, "crates", name, "src"))),
    )
  ).join("\n");
  for (const marker of ["@inflatable-cookie/poodle-", "SignalPlugin", "wgpu::", "PluginPayload"]) {
    if (sharedSource.includes(marker)) {
      throw new Error(`Rust artifacts contain forbidden product marker ${marker}`);
    }
  }

  await cp(join(proofRoot, "rust"), join(workspace, "consumers"), {
    recursive: true,
  });
  await writeFile(join(workspace, "Cargo.toml"), rustWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(workspace, "Cargo.lock"));

  const packages = {
    pure: "longhorn-native-content-pure-artifact-proof",
    nucleus: "longhorn-native-content-nucleus-artifact-proof",
    soundcheck: "longhorn-native-content-soundcheck-artifact-proof",
    jetstream: "longhorn-native-content-jetstream-artifact-proof",
  } as const;
  const expectedGraphs = {
    pure: ["longhorn-core", "longhorn-native-content"],
    nucleus: [
      "longhorn-core",
      "longhorn-native-content",
      "longhorn-tauri-native-content-child-view",
    ],
    soundcheck: [
      "longhorn-core",
      "longhorn-native-content",
      "longhorn-native-content-isolated-window",
    ],
    jetstream: [
      "longhorn-core",
      "longhorn-native-content",
      "longhorn-native-content-backing-surface",
    ],
  } as const;
  const traces: Record<string, Record<string, unknown>> = {};
  const graphs: Record<string, readonly string[]> = {};
  for (const shape of Object.keys(packages) as Array<keyof typeof packages>) {
    traces[shape] = parseTrace(
      await run(
        [
          "cargo",
          "+1.95.0",
          "run",
          "-p",
          packages[shape],
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
        packages[shape],
        "--offline",
        "--prefix",
        "none",
      ],
      workspace,
    );
    graphs[shape] = longhornPackages(tree);
    assertExactSet(`${shape} Rust graph`, graphs[shape], expectedGraphs[shape]);
  }
  if (!equalJson(traces.pure, traces.nucleus)) {
    throw new Error("pure child-view and selected child-view traces diverged");
  }
  return {
    identities,
    graphs,
    traces: traces as Record<Shape | "pure", Record<string, unknown>>,
    packaging: {
      privateSourceInventories: true,
      registryNormalizedCrates: false,
      cargoPackageListPassed: true,
    },
  };
}

async function verifyConsumer(
  shape: Shape,
  artifacts: ReadonlyMap<string, string>,
  native: Record<string, unknown>,
) {
  const source = join(proofRoot, "consumers", shape);
  const stage = join(temporaryRoot, `consumer-${shape}-${randomUUID()}`);
  await mkdir(join(stage, "consumers"), { recursive: true });
  await cp(source, join(stage, "consumers", shape), { recursive: true });
  await cp(join(proofRoot, "common.ts"), join(stage, "common.ts"));
  await cp(
    join(proofRoot, "consumer-tsconfig.json"),
    join(stage, "consumer-tsconfig.json"),
  );
  await writeFile(
    join(stage, "consumers", shape, "fixture.json"),
    `${JSON.stringify(native.rendererFixture, null, 2)}\n`,
  );

  const manifest = JSON.parse(
    await readFile(join(source, "package.json"), "utf8"),
  ) as PackageManifest;
  manifest.dependencies = rewriteDependencies(manifest.dependencies, artifacts);
  // Only Longhorn's own packs are overridden onto paths. Poodle is published,
  // so the staged consumer resolves it from the registry as a real consumer
  // does -- which is what the pack indirection stood in for before it shipped.
  manifest.overrides = Object.fromEntries(
    [...artifacts].map(([name, path]) => [name, fileDependency(path)]),
  );
  await writeFile(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  await run(["bun", "install", "--ignore-scripts"], stage);
  await run(["bun", "x", "tsc", "-p", "consumer-tsconfig.json"], stage);
  const usesSvelte = shape !== "soundcheck";
  if (usesSvelte) {
    await run(
      ["bun", "x", "svelte-check", "--tsconfig", "consumer-tsconfig.json"],
      stage,
    );
  }
  const renderer = parseTrace(
    await run(["bun", `consumers/${shape}/proof.ts`], stage),
  );
  if (!equalJson(renderer.publicTrace, native.publicTrace)) {
    throw new Error(`${shape} renderer trace diverged from packed Rust trace`);
  }

  // Card 164: soundcheck composes the framework tier only; the other two also
  // take the Svelte projection. The distinction survives the consolidation
  // because the projection tier is still a separate package.
  const expectedLonghorn =
    shape === "soundcheck"
      ? ["@inflatable-cookie/longhorn"]
      : [
          "@inflatable-cookie/longhorn",
          "@inflatable-cookie/longhorn-poodle-svelte",
        ];
  assertExactSet(
    `${shape} installed Longhorn packages`,
    (await installedScope(stage, "@inflatable-cookie")).filter((name) => name === "longhorn" || name.startsWith("longhorn-")).map((name) => `@inflatable-cookie/${name}`),
    expectedLonghorn,
  );
  for (const name of expectedLonghorn) await assertArtifactInstall(stage, name);
  for (const forbidden of [
    "@inflatable-cookie/longhorn-bridge",
    "@inflatable-cookie/longhorn-commands",
    "@inflatable-cookie/longhorn-config",
    "@inflatable-cookie/longhorn-history",
    "@inflatable-cookie/longhorn-layout",
    "@inflatable-cookie/longhorn-notifications",
    "@inflatable-cookie/longhorn-operation",
    "@inflatable-cookie/longhorn-settings",
    "@inflatable-cookie/longhorn-surfaces",
    "@inflatable-cookie/longhorn-tauri",
    "@inflatable-cookie/longhorn-transfer",
  ]) {
    await assertPackageAbsent(stage, forbidden);
  }

  if (usesSvelte) {
    for (const artifact of POODLE_RELEASE.packages) {
      await assertArtifactInstall(stage, artifact.name);
    }
    const svelte = await installedPackage(stage, "svelte");
    if (svelte.manifest.version !== "5.38.6") {
      throw new Error(`${shape} installed unexpected Svelte version`);
    }
    await assertSingleSvelteRuntime(stage);
  } else {
    assertExactSet(
      "Soundcheck installed Poodle packages",
      (await installedScope(stage, "@inflatable-cookie")).filter((name) =>
        name.startsWith("poodle-")
      ),
      [],
    );
    await assertPackageAbsent(stage, "svelte");
  }

  const expectedPermissions = usesSvelte
    ? [
        "read-native-content",
        "mutate-native-content",
        "core:event:allow-listen",
        "core:event:allow-unlisten",
      ]
    : [];
  const permissions = (
    JSON.parse(
      await readFile(join(source, "capability.json"), "utf8"),
    ) as { readonly permissions: readonly string[] }
  ).permissions;
  assertExactSet(`${shape} permissions`, permissions, expectedPermissions);

  const expectedImports = usesSvelte
    ? [
        "@inflatable-cookie/longhorn-poodle-svelte/native-content",
        "@inflatable-cookie/longhorn/native-content",
        "@inflatable-cookie/poodle-svelte",
      ]
    : ["@inflatable-cookie/longhorn/native-content"];
  assertExactSet(`${shape} imports`, await packageImports(stage), expectedImports);
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
    longhornPackages: expectedLonghorn,
    imports: expectedImports,
    permissions: expectedPermissions,
    poodleVersion: usesSvelte ? POODLE_RELEASE.version : null,
    svelteCompiled: usesSvelte,
    nativeRendererParity: true,
    cleanInstall: true,
    siblingSourceAliases: false,
    trace: renderer,
  };
}

async function verifyPackagedMechanisms() {
  const declarations = [
    {
      shape: "child_view",
      directory: "tauri-native-content-child-view-proof",
      reportOutcome: "pass_with_unmet_environment_claims",
      passing: 8,
      macos: "packaged-proof-pass-with-native-scale-switch-unmet",
      windows: "unproved",
      linux: "unproved",
      liveScale: "unproved",
    },
    {
      shape: "isolated_window",
      directory: "tauri-native-content-isolated-window-proof",
      reportOutcome: "pass",
      passing: 11,
      macos: "packaged-proof-pass",
      windows: "unsupported",
      linux: "unsupported",
      liveScale: "not_separately_claimed",
    },
    {
      shape: "backing_surface",
      directory: "tauri-native-content-backing-surface-proof",
      reportOutcome: "pass_with_unmet_environment_claims",
      passing: 10,
      macos: "packaged-proof-pass-with-native-scale-transition-unmet",
      windows: "unsupported",
      linux: "unsupported",
      liveScale: "unproved",
    },
  ] as const;
  const results = [];
  for (const declaration of declarations) {
    const root = join(repoRoot, "examples", declaration.directory, "evidence");
    const inventory = JSON.parse(
      await readFile(join(root, "inventory.json"), "utf8"),
    ) as {
      readonly packaged_run: {
        readonly report: string;
        readonly transcript: string;
        readonly outcome: string;
        readonly passed_checks: number;
      };
      readonly native_boundary: Record<string, unknown>;
      readonly forbidden_edges: Record<string, boolean>;
    };
    if (
      inventory.packaged_run.outcome !== declaration.reportOutcome ||
      inventory.packaged_run.passed_checks !== declaration.passing
    ) {
      throw new Error(`${declaration.shape} packaged inventory regressed`);
    }
    if (Object.values(inventory.forbidden_edges).some(Boolean)) {
      throw new Error(`${declaration.shape} packaged graph has a forbidden edge`);
    }
    const report = JSON.parse(
      await readFile(join(root, inventory.packaged_run.report.replace("evidence/", "")), "utf8"),
    ) as {
      readonly outcome: string;
      readonly checks: readonly { readonly id: string; readonly status: string }[];
    };
    if (report.outcome !== declaration.reportOutcome) {
      throw new Error(`${declaration.shape} packaged report outcome regressed`);
    }
    const passing = report.checks.filter(({ status }) => status === "pass").length;
    if (passing !== declaration.passing) {
      throw new Error(`${declaration.shape} passing check count regressed`);
    }
    const transcript = await readFile(
      join(root, inventory.packaged_run.transcript.replace("evidence/", "")),
      "utf8",
    );
    if (/fixture_error|proof_failed/.test(transcript)) {
      throw new Error(`${declaration.shape} transcript contains failure evidence`);
    }
    const support = JSON.parse(
      await readFile(join(root, "target-support.json"), "utf8"),
    ) as {
      readonly targets: Record<string, { readonly status: string }>;
    };
    for (const target of ["macos", "windows", "linux"] as const) {
      if (support.targets[target]?.status !== declaration[target]) {
        throw new Error(`${declaration.shape} ${target} support status changed`);
      }
    }
    results.push({
      shape: declaration.shape,
      packagedOutcome: declaration.reportOutcome,
      passingChecks: declaration.passing,
      support: {
        macos: "supported",
        windows: declaration.windows,
        linux: declaration.linux,
      },
      liveScaleTransition: declaration.liveScale,
      nativeBoundary: inventory.native_boundary,
      forbiddenEdges: inventory.forbidden_edges,
      transcriptFailures: false,
    });
  }
  return results;
}

async function verifyPrototypeDisposition() {
  const roots = [
    "native-content",
    "native-content-child-webview",
    "native-content-isolated-window",
    "native-content-backing-surface",
  ];
  for (const root of roots) {
    const manifest = await readFile(
      join(repoRoot, "prototypes", root, "Cargo.toml"),
      "utf8",
    );
    if (!manifest.includes("publish = false")) {
      throw new Error(`${root} prototype is not explicitly non-publishable`);
    }
  }
  const workspace = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  const members = workspace.slice(
    workspace.indexOf("members = ["),
    workspace.indexOf("]", workspace.indexOf("members = [")) + 1,
  );
  if (members.includes("prototypes/native-content")) {
    throw new Error("native-content prototype entered the production workspace");
  }
  return {
    disposition: "retained_non_publishable_evidence",
    productionWorkspaceMembers: false,
    publishable: false,
    compatibilityAuthority: false,
    sourcePromotedDirectly: false,
  };
}

function rustWorkspaceManifest(): string {
  return `[workspace]
members = ["consumers/common", "consumers/pure", "consumers/nucleus", "consumers/soundcheck", "consumers/jetstream", "crates/longhorn-core", "crates/longhorn-native-content", "crates/longhorn-tauri-native-content-child-view", "crates/longhorn-native-content-isolated-window", "crates/longhorn-native-content-backing-surface"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT"
repository = "https://github.com/inflatable-cookie/longhorn"

[workspace.dependencies]
longhorn-core = { path = "crates/longhorn-core", version = "0.1.0" }
longhorn-native-content = { path = "crates/longhorn-native-content", version = "0.1.0" }
longhorn-tauri-native-content-child-view = { path = "crates/longhorn-tauri-native-content-child-view", version = "0.1.0" }
longhorn-native-content-isolated-window = { path = "crates/longhorn-native-content-isolated-window", version = "0.1.0" }
longhorn-native-content-backing-surface = { path = "crates/longhorn-native-content-backing-surface", version = "0.1.0" }
longhorn-native-content-artifact-proof-common = { path = "consumers/common" }
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

function rewriteDependencies(
  dependencies: Record<string, string>,
  artifacts: ReadonlyMap<string, string>,
) {
  return Object.fromEntries(
    Object.entries(dependencies).map(([name, version]) => [
      name,
      artifacts.has(name) ? fileDependency(artifacts.get(name)!) : version,
    ]),
  );
}

function longhornPackages(tree: string): readonly string[] {
  return [
    ...new Set(
      tree
        .split("\n")
        .map((line) => line.trim().split(/\s+/)[0] ?? "")
        .filter(
          (name) =>
            name.startsWith("longhorn-") && !name.includes("artifact-proof"),
        ),
    ),
  ].sort();
}

async function packageImports(stage: string) {
  const files = (await readdir(stage, { recursive: true })).filter(
    (path) =>
      /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules/"),
  );
  const imports = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(stage, path), "utf8");
    for (const match of source.matchAll(/from\s+["']((?:@inflatable-cookie\/(?:longhorn[/-]|poodle-))[^"']+)["']/g)) {
      imports.add(match[1]!);
    }
  }
  return [...imports].sort();
}

async function installedScope(stage: string, scope: string) {
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
      throw new Error(`${name} resolved to sibling source: ${installed.realPath}`);
    }
  }
  if (installed.manifest.version !== "0.1.0") {
    throw new Error(`${name} installed unexpected version`);
  }
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

async function assertPackageAbsent(stage: string, name: string) {
  try {
    await lstat(join(stage, "node_modules", ...name.split("/")));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the install graph`);
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
    throw new Error(`expected one Svelte runtime, found ${manifests.length}`);
  }
}

async function readSourceTree(root: string) {
  const files = (await readdir(root, { recursive: true }))
    .filter((path) => /\.(rs|ts|svelte)$/.test(path))
    .sort();
  return (
    await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))
  ).join("\n");
}

async function run(command: readonly string[], cwd: string) {
  const subprocess = Bun.spawn(command, {
    cwd,
    env: { ...process.env, COPYFILE_DISABLE: "1" },
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

function parseTrace(output: string): Record<string, unknown> {
  const line = output
    .split("\n")
    .find((candidate) => candidate.trim().startsWith("{"));
  if (line === undefined) throw new Error("proof emitted no JSON trace");
  return JSON.parse(line) as Record<string, unknown>;
}

function assertExactSet(
  label: string,
  actual: readonly string[],
  expected: readonly string[],
) {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(`${label} mismatch: ${left.join(", ")} != ${right.join(", ")}`);
  }
}

function equalJson(left: unknown, right: unknown) {
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

function fileDependency(path: string) {
  return `file:${path}`;
}

async function digest(path: string) {
  return createHash("sha256").update(await readFile(path)).digest("hex");
}
