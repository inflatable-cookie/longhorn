import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const soundcheckRoot = resolve(
  process.env.SOUNDCHECK_REPO ?? resolve(longhornRoot, "../soundcheck"),
);
const siblingRoots = {
  soundcheck_library: resolve(longhornRoot, "../soundcheck-library"),
  signal: resolve(longhornRoot, "../signal"),
  poodle: resolve(longhornRoot, "../poodle"),
  swallowtail: resolve(longhornRoot, "../swallowtail"),
};
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/soundcheck-card119/conformance-closeout-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifySources();
verifyPriorCards();
const artifacts = verifyPrivateArtifacts();
const product = verifyProductProfile();
const composition = verifyComposition();
const matrices = verifyMatrices();
const duplicates = verifyDuplicateAndAdapterPosture();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: fixture.outcome,
      sources: fixture.sources,
      artifacts,
      product,
      composition,
      matrices,
      duplicates,
      authority: fixture.authority,
      rollback: fixture.rollback,
      native: fixture.native,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  equal(
    fixture.schema,
    "longhorn.soundcheck-migration-conformance-closeout.v1",
    "fixture schema",
  );
  equal(
    fixture.outcome,
    "pass-with-environment-unmet-native-gui",
    "fixture outcome",
  );
  equal(
    fixture.composition.canonical_application_id,
    "com.inflatablecookie.soundcheck",
    "canonical application id",
  );
  equal(fixture.composition.hierarchy, ["display", "window"], "hierarchy");
  assert(
    !fixture.composition.package_manager_publication,
    "package-manager publication claimed",
  );
  assert(!fixture.rollback.dual_write, "rollback used dual-write authority");
  assert(!fixture.rollback.silent_fallback, "rollback used silent fallback");
  assert(
    !fixture.rollback.legacy_source_cleanup_authorized,
    "legacy cleanup authority was fabricated",
  );
  assert(
    fixture.rollback.temporary_worktree_removed,
    "previous-build worktree remains",
  );
  equal(
    fixture.rollback.database_semantic_before_sha256,
    fixture.rollback.database_semantic_after_sha256,
    "rollback database semantic digest",
  );
  equal(
    fixture.rollback.settings_before_sha256,
    fixture.rollback.settings_after_sha256,
    "rollback settings digest",
  );
  assert(
    fixture.rollback.previous_process_reached_stable_run_loop,
    "previous build did not reach its run loop",
  );
  assert(
    fixture.rollback.profile_verified_after_readback,
    "profile was not verified after previous-build readback",
  );
  equal(fixture.native.platform, "macos", "native proof platform");
  assert(fixture.native.bundle_built, "acceptance bundle was not built");
  assert(
    fixture.native.host_bypass_started &&
      fixture.native.window_restore_converged_attempt === 1,
    "native startup and restore evidence is incomplete",
  );
  equal(
    fixture.native.live_plugin_gui_matrix,
    "environment-unmet-computer-control-timeout",
    "native GUI classification",
  );
  assert(!fixture.native.silent_degradation, "native-content degraded silently");
  assert(!fixture.validation.live_data_mutated, "live product data mutation claimed");
}

function verifySources(): void {
  equal(
    git(soundcheckRoot, ["rev-parse", "HEAD"]),
    fixture.sources.soundcheck_commit,
    "Soundcheck HEAD",
  );
  assertClean("Soundcheck", soundcheckRoot);
  for (const [name, root] of Object.entries(siblingRoots)) {
    equal(
      git(root, ["rev-parse", "HEAD"]),
      fixture.sources[`${name}_commit`],
      `${name} HEAD`,
    );
    assertClean(name, root);
  }
  git(longhornRoot, [
    "cat-file",
    "-e",
    `${fixture.sources.longhorn_selected_source_commit}^{commit}`,
  ]);
  git(soundcheckRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.soundcheck_previous_authority_commit,
    fixture.sources.soundcheck_commit,
  ]);
  equal(
    sha256(readFileSync(resolve(soundcheckRoot, "Cargo.lock"))),
    fixture.sources.soundcheck_cargo_lock_sha256,
    "Soundcheck Cargo lock",
  );
  equal(
    sha256(readFileSync(resolve(soundcheckRoot, "bun.lock"))),
    fixture.sources.soundcheck_bun_lock_sha256,
    "Soundcheck Bun lock",
  );
}

function verifyPriorCards(): void {
  for (const card of ["115", "116", "117", "118"]) {
    const proof = JSON.parse(
      command(longhornRoot, ["bun", `scripts/verify-soundcheck-card${card}.ts`]),
    );
    assert(
      String(proof.outcome).startsWith("pass"),
      `Card ${card} evidence does not pass`,
    );
  }
}

function verifyPrivateArtifacts(): Record<string, unknown> {
  const proof = JSON.parse(
    command(
      longhornRoot,
      ["bun", "scripts/verify-secondary-consumer-private-artifacts.ts"],
      {
        SECONDARY_CONSUMER_ADMISSION_FIXTURE:
          fixture.validation.private_artifact_fixture,
      },
    ),
  );
  equal(proof.outcome, "pass", "private artifact proof");
  equal(
    proof.sources.consumers.soundcheck,
    fixture.sources.soundcheck_commit,
    "artifact Soundcheck source",
  );
  assert(proof.audits.oneSvelteRuntimePerRenderer, "duplicate Svelte runtime");
  assert(
    proof.audits.isolatedProducedArtifactInstalls,
    "artifacts were not installed in isolation",
  );
  assert(
    !proof.audits.packageManagerPublication,
    "artifact proof published packages",
  );
  return {
    typescript: proof.artifacts.longhornTypescriptArtifactSet,
    rust: proof.artifacts.longhornRustArtifactSet,
    poodle: proof.artifacts.poodleArtifactSet,
    oneSvelteRuntime: true,
    isolatedInstalls: true,
  };
}

function verifyProductProfile(): Record<string, unknown> {
  const output = command(soundcheckRoot, [
    "effigy",
    "product:acceptance:verify",
  ]);
  for (const token of [
    "portable=6",
    "hosted=1",
    "native=3",
    "plans=7/4/1",
  ]) {
    assert(output.includes(token), `product profile lost ${token}`);
  }
  return fixture.validation.product_profile;
}

function verifyComposition(): Record<string, unknown> {
  const manifest = JSON.parse(soundcheckSource("package.json")) as {
    dependencies: Record<string, string>;
  };
  for (const name of fixture.composition.typescript_packages) {
    assert(
      manifest.dependencies[name]?.startsWith("file:../longhorn/"),
      `${name} is not selected from private Longhorn source`,
    );
  }
  for (const forbidden of [
    "@longhorn/layout",
    "@longhorn/surfaces",
    "@longhorn/transfer",
    "@longhorn/surface-transfer",
    "@longhorn/commands",
    "@longhorn/history",
    "@longhorn/bridge",
    "@longhorn/notifications",
    "@longhorn/native-content-svelte",
  ]) {
    assert(!manifest.dependencies[forbidden], `renderer graph resolves ${forbidden}`);
  }

  const workspace = soundcheckSource("Cargo.toml");
  const desktop = soundcheckSource("src-tauri/Cargo.toml");
  for (const name of fixture.composition.rust_packages) {
    assert(
      workspace.includes(`${name} = { path = "../longhorn/crates/${name}" }`),
      `workspace dependency missing ${name}`,
    );
    assert(desktop.includes(`${name}.workspace = true`), `desktop dependency missing ${name}`);
  }
  const cargoTree = command(soundcheckRoot, [
    "cargo",
    "tree",
    "-p",
    "soundcheck-app",
    "--edges",
    "normal",
    "--prefix",
    "none",
  ]);
  for (const forbidden of [
    "longhorn-layout ",
    "longhorn-surfaces ",
    "longhorn-transfer ",
    "longhorn-surface-transfer ",
    "longhorn-command ",
    "longhorn-history ",
    "longhorn-bridge ",
    "longhorn-notifications ",
    "longhorn-native-content-backing-surface ",
    "longhorn-tauri-native-content-child-view ",
  ]) {
    assert(!cargoTree.includes(forbidden), `Rust graph resolves ${forbidden.trim()}`);
  }

  const tauri = JSON.parse(soundcheckSource("src-tauri/tauri.conf.json"));
  equal(
    tauri.identifier,
    fixture.composition.canonical_application_id,
    "Tauri identifier",
  );
  const capability = JSON.parse(
    soundcheckSource("src-tauri/capabilities/main.json"),
  );
  equal(capability.identifier, fixture.capability.identifier, "capability id");
  equal(capability.windows, fixture.capability.windows, "capability windows");
  equal(
    capability.permissions,
    fixture.capability.permissions,
    "capability permissions",
  );
  assert(!("remote" in capability), "remote capability selector exists");
  assert(
    capability.windows.every((window: string) => !window.includes("plugin")),
    "plugin helper entered Tauri window capability",
  );

  return {
    hierarchy: fixture.composition.hierarchy,
    rustPackages: fixture.composition.rust_packages.length,
    typescriptPackages: fixture.composition.typescript_packages.length,
    forbiddenSystemsAbsent: fixture.composition.forbidden_systems,
    capability: fixture.capability.identifier,
    capabilityWindows: fixture.capability.windows,
  };
}

function verifyMatrices(): Record<string, number> {
  const locations: Record<string, string[]> = {
    storage: [
      "src-tauri/src/storage_migration/mod.rs",
      "src-tauri/src/storage_profile.rs",
    ],
    settings_recovery: [
      "src-tauri/src/settings_host.rs",
      "src-tauri/src/config_operations.rs",
    ],
    scan: ["src-tauri/src/operation_host.rs"],
    helper: ["src-tauri/src/plugin_inspection_process.rs"],
  };
  const longhornWindowSources = [
    "crates/longhorn-tauri-windowing/tests/composition/fixtures.rs",
    "crates/longhorn-tauri-windowing/tests/lifecycle/persistence.rs",
    "crates/longhorn-windowing-config/tests/config_sink.rs",
  ].map(longhornSource);
  const counts: Record<string, number> = {};
  for (const [group, tests] of Object.entries(fixture.matrices)) {
    const sources =
      group === "window"
        ? longhornWindowSources
        : (locations[group] ?? []).map(soundcheckSource);
    for (const test of tests) {
      assert(
        sources.some((source) => source.includes(`fn ${test}`)),
        `${group} matrix lost ${test}`,
      );
    }
    counts[group] = tests.length;
  }
  return counts;
}

function verifyDuplicateAndAdapterPosture(): Record<string, unknown> {
  const app = soundcheckSource("src-tauri/src/lib.rs");
  for (const forbidden of [
    "app_settings::set_agent_review_settings",
    "library_commands::cancel_plugin_scan",
    "library_commands::list_library_backups",
    "library_commands::restore_library_backup,",
    "longhorn_tauri_native_content::longhorn_native_content_mutate",
  ]) {
    assert(!app.includes(forbidden), `superseded command remains registered: ${forbidden}`);
  }
  for (const required of [
    "longhorn_tauri_settings::longhorn_settings_registry",
    "longhorn_tauri_settings::longhorn_settings_apply",
    "longhorn_tauri_operation::longhorn_operation_snapshot",
    "longhorn_tauri_operation::longhorn_operation_mutate",
    "longhorn_tauri_operation::longhorn_operation_cancel",
    "plugin_inspection_coordination::longhorn_native_content_connect",
    "plugin_inspection_coordination::longhorn_native_content_snapshot",
  ]) {
    assert(app.includes(required), `selected protocol command is absent: ${required}`);
  }
  const retained = [
    ["src-tauri/src/config_operations.rs", "SoundcheckConfigOperationsAuthority"],
    ["src-tauri/src/operation_host.rs", "SoundcheckScanExecutor"],
    ["src-tauri/src/plugin_inspection_coordination.rs", "PluginInspectionProcessRuntime"],
    ["src-tauri/src/plugin_inspection.rs", "InProcessVst3Processor"],
    ["src-tauri/src/reaper_inspection.rs", "ReaperInspectionService"],
    ["src-tauri/src/keepsake_settings.rs", "KEEPSAKE"],
  ] as const;
  for (const [path, token] of retained) {
    assert(
      soundcheckSource(path).includes(token),
      `retained adapter lost: ${path} ${token}`,
    );
  }
  return {
    genericAuthoritiesPerMigratedConcern: 1,
    supersededCommandsAbsent: 5,
    classifiedProductAdaptersPresent: retained.length,
    productPayloadInLonghorn: false,
  };
}

function soundcheckSource(path: string): string {
  return readFileSync(resolve(soundcheckRoot, path), "utf8");
}

function longhornSource(path: string): string {
  return git(longhornRoot, [
    "show",
    `${fixture.sources.longhorn_selected_source_commit}:${path}`,
  ]);
}

function assertClean(label: string, root: string): void {
  const status = git(root, ["status", "--porcelain"]);
  assert(!status, `${label} worktree is dirty:\n${status}`);
}

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function command(
  root: string,
  arguments_: string[],
  environment: Record<string, string> = {},
): string {
  const result = spawnSync(arguments_[0], arguments_.slice(1), {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...environment },
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || `${arguments_.join(" ")} failed`);
  }
  return result.stdout.trim();
}

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface Fixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: Record<string, string>;
  readonly composition: {
    readonly canonical_application_id: string;
    readonly hierarchy: string[];
    readonly rust_packages: string[];
    readonly typescript_packages: string[];
    readonly forbidden_systems: string[];
    readonly package_manager_publication: boolean;
  };
  readonly capability: {
    readonly identifier: string;
    readonly windows: string[];
    readonly permissions: string[];
  };
  readonly matrices: Record<string, string[]>;
  readonly authority: Record<string, string[]>;
  readonly rollback: Record<string, string | boolean> & {
    readonly database_semantic_before_sha256: string;
    readonly database_semantic_after_sha256: string;
    readonly settings_before_sha256: string;
    readonly settings_after_sha256: string;
    readonly dual_write: boolean;
    readonly silent_fallback: boolean;
    readonly legacy_source_cleanup_authorized: boolean;
    readonly temporary_worktree_removed: boolean;
    readonly previous_process_reached_stable_run_loop: boolean;
    readonly profile_verified_after_readback: boolean;
  };
  readonly native: Record<string, string | boolean | number | string[]> & {
    readonly platform: string;
    readonly bundle_built: boolean;
    readonly host_bypass_started: boolean;
    readonly window_restore_converged_attempt: number;
    readonly live_plugin_gui_matrix: string;
    readonly silent_degradation: boolean;
  };
  readonly validation: {
    readonly product_profile: Record<string, unknown>;
    readonly private_artifact_fixture: string;
    readonly live_data_mutated: boolean;
  };
}
