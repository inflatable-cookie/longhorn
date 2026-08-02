import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: {
    longhorn_prior_commit: string;
    longhorn_cutover_commit: string;
    soundcheck_prior_commit: string;
    soundcheck_cutover_commit: string;
    soundcheck_lock_sha256: string;
  };
  identity: {
    canonical_application_id: string;
    stable_storage_name: string;
    profile: string;
    locator_identity: string;
    effective_leaf: string;
  };
  layout: {
    database_relative_path: string;
    database_data_root_override: string;
    established_database_moved: boolean;
    same_layout_unknown_source_inventory: boolean;
  };
  domains: {
    application_settings: {
      id: string;
      storage_class: string;
      relative_path: string;
    };
    window_placement: {
      id: string;
      storage_class: string;
      relative_path: string;
    };
    library_database: {
      id: string;
      authority: string;
      adapter: string;
      capture: string;
      restore: string;
    };
  };
  migration: Record<string, boolean | string>;
  window: {
    logical_id: string;
    tauri_label: string;
    protected_primary: boolean;
    dynamic_factory: boolean;
    hidden_startup: boolean;
    minimum_size: [number, number];
    debounce_ms: number;
    close_flush_timeout_ms: number;
    reveal_signals: string[];
  };
  overrides: {
    test_confined: boolean;
    all_paths_must_remain_inside_proof_root: boolean;
  };
  packages: {
    required_rust: string[];
    surface_packages: boolean;
    settings_packages: boolean;
    operation_packages: boolean;
    native_content_packages: boolean;
    package_manager_publication: boolean;
  };
  validation: { native_gui_proof: string; unrelated_failure: string };
};

const longhornRoot = resolve(import.meta.dir, "..");
const soundcheckRoot = resolve(
  process.env.SOUNDCHECK_REPO ?? resolve(longhornRoot, "../soundcheck"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/soundcheck-card115/storage-config-window-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;
const longhornCommit = fixture.sources.longhorn_cutover_commit;
const soundcheckCommit = fixture.sources.soundcheck_cutover_commit;

verifyReceipts();
verifyFixture();
verifySoundcheckCutover();
verifyLonghornGapClosure();

console.log(
  JSON.stringify(
    {
      schema:
        "longhorn.soundcheck-storage-config-window-cutover-verification.v1",
      outcome: "pass",
      longhornCommit,
      soundcheckCommit,
      identity: fixture.identity,
      domains: Object.values(fixture.domains).map((domain) => domain.id),
      window: {
        logicalId: fixture.window.logical_id,
        transportLabel: fixture.window.tauri_label,
        revealSignals: fixture.window.reveal_signals,
      },
      retainedDatabaseAuthority: fixture.domains.library_database.authority,
      nativeGuiProof: fixture.validation.native_gui_proof,
      packageManagerPublication: fixture.packages.package_manager_publication,
    },
    null,
    2,
  ),
);

function verifyReceipts(): void {
  for (const [root, commit] of [
    [longhornRoot, longhornCommit],
    [soundcheckRoot, soundcheckCommit],
  ] as const) {
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  git(longhornRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.longhorn_prior_commit,
    longhornCommit,
  ]);
  git(soundcheckRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.soundcheck_prior_commit,
    soundcheckCommit,
  ]);
  assertEqual(
    createHash("sha256").update(soundcheckReadRaw("Cargo.lock")).digest("hex"),
    fixture.sources.soundcheck_lock_sha256,
    "Soundcheck lockfile digest",
  );
}

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.soundcheck-storage-config-window-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(
    fixture.identity.canonical_application_id,
    "com.inflatablecookie.soundcheck",
    "canonical id",
  );
  assertEqual(
    fixture.identity.stable_storage_name,
    "Soundcheck",
    "stable storage name",
  );
  assertEqual(
    fixture.identity.profile,
    "shared-product-root-v1",
    "storage profile",
  );
  assertEqual(
    fixture.identity.locator_identity,
    "canonical-application-id",
    "locator identity",
  );
  assertEqual(
    fixture.layout.database_relative_path,
    "library.db",
    "database path",
  );
  assertEqual(
    fixture.layout.database_data_root_override,
    "established-product-root",
    "database root policy",
  );
  assert(
    !fixture.layout.established_database_moved,
    "established database moved",
  );
  assert(
    !fixture.layout.same_layout_unknown_source_inventory,
    "same-layout source inventory enabled",
  );
  assertEqual(
    fixture.domains.application_settings.storage_class,
    "user-config",
    "settings storage class",
  );
  assertEqual(
    fixture.domains.window_placement.storage_class,
    "machine-state",
    "window storage class",
  );
  assertEqual(
    fixture.domains.library_database.authority,
    "soundcheck-library",
    "database authority",
  );
  assert(
    fixture.migration.verified_backup_before_publish === true,
    "backup ordering",
  );
  assert(fixture.migration.exact_digest_receipt === true, "digest receipt");
  assert(fixture.migration.locator_last === true, "locator ordering");
  assert(fixture.migration.source_retained === true, "source retention");
  assert(
    fixture.migration.cleanup_authorized === false,
    "cleanup authorization",
  );
  assert(fixture.migration.dual_write === false, "dual write");
  assert(fixture.migration.silent_fallback === false, "silent fallback");
  assertEqual(fixture.window.minimum_size.join("x"), "320x240", "minimum size");
  assertEqual(fixture.window.debounce_ms, 300, "window debounce");
  assertEqual(
    fixture.window.close_flush_timeout_ms,
    2000,
    "close flush timeout",
  );
  assertEqual(fixture.window.reveal_signals.length, 2, "reveal signal count");
  assert(fixture.overrides.test_confined, "overrides are not test-confined");
  assert(
    fixture.overrides.all_paths_must_remain_inside_proof_root,
    "proof-root containment",
  );
  for (const selected of [
    fixture.packages.surface_packages,
    fixture.packages.settings_packages,
    fixture.packages.operation_packages,
    fixture.packages.native_content_packages,
    fixture.packages.package_manager_publication,
  ]) {
    assert(!selected, "unadmitted package or publication edge");
  }
  assertEqual(
    fixture.validation.native_gui_proof,
    "deferred-to-card119",
    "native proof posture",
  );
}

function verifySoundcheckCutover(): void {
  const workspace = soundcheckRead("Cargo.toml");
  const desktopCargo = soundcheckRead("src-tauri/Cargo.toml");
  for (const dependency of fixture.packages.required_rust) {
    assertContains(
      workspace,
      `${dependency} = { path = \"../longhorn/crates/${dependency}\" }`,
      "workspace dependency",
    );
    assertContains(
      desktopCargo,
      `${dependency}.workspace = true`,
      "desktop dependency",
    );
  }
  for (const excluded of [
    "longhorn-surface",
    "longhorn-settings",
    "longhorn-operation",
    "longhorn-native-content",
  ]) {
    assertExcludes(desktopCargo, excluded, "desktop dependency");
  }

  const tauri = JSON.parse(soundcheckRead("src-tauri/tauri.conf.json")) as {
    identifier: string;
    app: { windows: Array<{ label?: string; visible: boolean }> };
  };
  assertEqual(
    tauri.identifier,
    fixture.identity.canonical_application_id,
    "Tauri identifier",
  );
  assertEqual(
    tauri.app.windows[0]?.label ?? "main",
    fixture.window.tauri_label,
    "Tauri window label",
  );
  assertEqual(
    tauri.app.windows[0]?.visible,
    !fixture.window.hidden_startup,
    "hidden startup",
  );

  const profile = soundcheckRead("src-tauri/src/storage_profile.rs");
  for (const token of [
    'CANONICAL_APPLICATION_ID: &str = "com.inflatablecookie.soundcheck"',
    'STABLE_STORAGE_NAME: &str = "Soundcheck"',
    "StorageProfileSelection::shared_product()",
    "StorageProfile::SharedProductRootV1",
    "with(RootKind::Data, database_root",
    "SOUNDCHECK_ACCEPTANCE_ROOT",
    "SOUNDCHECK_ASSISTANT_PROOF_ROOT",
    "are test-confined and require an explicit proof root",
    "must remain inside the proof root",
  ]) {
    assertContains(profile, token, "storage profile");
  }

  const migration = soundcheckRead("src-tauri/src/storage_migration/mod.rs");
  for (const token of [
    'TRANSITION_ID: &str = "soundcheck-storage-profile-card115-v1"',
    '"soundcheck.library-database"',
    "SoundcheckSqliteAdapter::new",
    "publish_verified_backup",
    "source_sha256",
    "app_settings_sha256",
    "window_placement_sha256",
    "cleanup_authorized: false",
    "plan.confirmation_digest()",
    "write_adoption_receipt",
    "conflicts with retained legacy source",
  ]) {
    assertContains(migration, token, "storage migration");
  }

  const sqlite = soundcheckRead("src-tauri/src/storage_migration/sqlite.rs");
  for (const token of [
    'BackupAdapterId::new("soundcheck-library-sqlite-online-v1")',
    '"sqlite-online-backup-api"',
    "BackupAdapterRestoreParticipation::FailureAtomic",
    ".backup(MAIN_DB, &snapshot, None)",
    ".restore(MAIN_DB, &source",
    '"PRAGMA quick_check"',
  ]) {
    assertContains(sqlite, token, "SQLite adapter");
  }

  const settings = soundcheckRead("src-tauri/src/app_settings.rs");
  for (const token of [
    'SETTINGS_DOMAIN_ID: &str = "soundcheck.app-settings"',
    'SETTINGS_DOMAIN_FILE: &str = "app-settings.json"',
    "StorageClass::UserConfig",
    "ConfigStore",
  ]) {
    assertContains(settings, token, "application settings domain");
  }
  for (const retired of [
    "WINDOW_SAVE_DEBOUNCE",
    "WINDOW_STATE_SENDER",
    "restore_main_window",
    "handle_main_window_event",
  ]) {
    assertExcludes(settings, retired, "retired window settings worker");
  }

  const domain = soundcheckRead("src-tauri/src/window_host/domain.rs");
  for (const token of [
    'DOMAIN_ID: &str = "soundcheck.window-placement"',
    'DOMAIN_FILE: &str = "window-placement.json"',
    "StorageClass::MachineState",
    "KnownDisplayRegistry",
    "SavedWindowPlacement",
  ]) {
    assertContains(domain, token, "window domain");
  }

  const host = soundcheckRead("src-tauri/src/window_host/mod.rs");
  for (const token of [
    'PRIMARY_WINDOW_ID: &str = "window:primary"',
    'MAIN_LABEL: &str = "main"',
    "assemble_tauri_window_host",
    "NoWindowFactory",
    "ProtectedPrimaryPolicy::Preserve",
    "ScreenSize::new(320, 240)",
    "WindowLifecycleDuration::from_millis(300)",
    "WindowLifecycleDuration::from_millis(2_000)",
    ".for_hidden_restore()",
    "mark_page_ready",
    "TauriAsyncWindowLifecycleScheduler",
    "ConfigWindowPlacementSink",
  ]) {
    assertContains(host, token, "window host");
  }

  const desktop = soundcheckRead("src-tauri/src/lib.rs");
  for (const token of [
    "window_host::install",
    "window_host::teardown",
    "soundcheck_window_page_ready",
  ]) {
    assertContains(desktop, token, "desktop assembly");
  }
  assertExcludes(
    desktop,
    "app_settings::handle_main_window_event",
    "old window event route",
  );
  assertContains(
    soundcheckRead("src/lib/api.ts"),
    'invoke("soundcheck_window_page_ready")',
    "renderer ready signal",
  );
}

function verifyLonghornGapClosure(): void {
  const inventory = longhornRead(
    "crates/longhorn-config/src/storage_layout/transition/inventory.rs",
  );
  for (const token of [
    "source_layout.digest() == request.target_layout.digest()",
    "Profile adoption changes no file authority",
    "Vec::new()",
  ]) {
    assertContains(inventory, token, "same-layout inventory bound");
  }
  const tests = longhornRead(
    "crates/longhorn-config/tests/storage_layout/transition.rs",
  );
  assertContains(
    tests,
    "same_layout_adoption_allows_derived_workspace_under_state",
    "same-layout regression",
  );
  assertContains(
    tests,
    "preview.source_unknown().is_empty()",
    "same-layout source assertion",
  );
}

function soundcheckRead(path: string): string {
  return git(soundcheckRoot, ["show", `${soundcheckCommit}:${path}`]);
}

function soundcheckReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${soundcheckCommit}:${path}`], {
    cwd: soundcheckRoot,
  });
}

function longhornRead(path: string): string {
  return git(longhornRoot, ["show", `${longhornCommit}:${path}`]);
}

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
  }).trim();
}

function assertContains(value: string, token: string, label: string): void {
  assert(value.includes(token), `${label} lacks ${JSON.stringify(token)}`);
}

function assertExcludes(value: string, token: string, label: string): void {
  assert(!value.includes(token), `${label} retains ${JSON.stringify(token)}`);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  assert(
    Object.is(actual, expected),
    `${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
