import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  registry: {
    generation: number;
    module: string;
    section: string;
    scope: string;
    apply_unit: string;
    timing: string;
    reset_supported: boolean;
    product_pages: string[];
    shared_pages: string[];
    capabilities: string[];
  };
  settings: Record<string, boolean | string[]>;
  backup_restore: Record<string, boolean | string | string[]>;
  unsupported: Record<string, boolean>;
  packages: {
    rust: string[];
    typescript: string[];
    package_manager_publication: boolean;
  };
  validation: Record<string, boolean | number | string>;
};

const longhornRoot = resolve(import.meta.dir, "..");
const soundcheckRoot = resolve(
  process.env.SOUNDCHECK_REPO ?? resolve(longhornRoot, "../soundcheck"),
);
const libraryRoot = resolve(
  process.env.SOUNDCHECK_LIBRARY_REPO ??
    resolve(longhornRoot, "../soundcheck-library"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/soundcheck-card116/settings-backup-recovery-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;
const longhornCommit = fixture.sources.longhorn_shell_commit;
const soundcheckCommit = fixture.sources.soundcheck_cutover_commit;

verifyReceipts();
verifyFixture();
verifyLonghornShell();
verifySoundcheckRegistryAndHost();
verifyRendererCutover();

console.log(
  JSON.stringify(
    {
      schema: "longhorn.soundcheck-settings-backup-recovery-verification.v1",
      outcome: "pass",
      longhornCommit,
      soundcheckCommit,
      pages: [
        ...fixture.registry.product_pages,
        ...fixture.registry.shared_pages,
      ],
      capabilities: fixture.registry.capabilities,
      retainedAuthority: fixture.backup_restore.authority,
      terminalOutcomes: fixture.backup_restore.terminal_outcomes,
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
    [libraryRoot, fixture.sources.soundcheck_library_commit],
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
    sha256(soundcheckReadRaw("Cargo.lock")),
    fixture.sources.soundcheck_cargo_lock_sha256,
    "Soundcheck Cargo lock digest",
  );
  assertEqual(
    sha256(soundcheckReadRaw("bun.lock")),
    fixture.sources.soundcheck_bun_lock_sha256,
    "Soundcheck Bun lock digest",
  );
  git(libraryRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.soundcheck_library_commit,
    "HEAD",
  ]);
}

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.soundcheck-settings-backup-recovery-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(fixture.registry.generation, 1, "registry generation");
  assertEqual(fixture.registry.timing, "immediate", "mutation timing");
  assert(!fixture.registry.reset_supported, "reset is unexpectedly supported");
  assertEqual(fixture.registry.product_pages.length, 5, "product page count");
  assertEqual(fixture.registry.shared_pages.length, 3, "shared page count");
  assertEqual(fixture.registry.capabilities.length, 4, "capability count");
  assert(fixture.settings.pair_is_atomic === true, "agent pair atomicity");
  assert(
    fixture.settings.product_validation_retained === true,
    "product validation authority",
  );
  assert(
    fixture.backup_restore.renderer_filesystem_authority === false,
    "renderer filesystem authority",
  );
  assert(
    fixture.backup_restore.schema_authority_moved === false,
    "SQLite schema authority moved",
  );
  for (const unsupported of Object.values(fixture.unsupported)) {
    assert(unsupported, "an unsupported capability was claimed");
  }
  assert(
    !fixture.packages.package_manager_publication,
    "package-manager publication claimed",
  );
  assert(
    fixture.validation.live_data_mutated === false,
    "live Soundcheck data mutation claimed",
  );
}

function verifyLonghornShell(): void {
  const shell = longhornRead(
    "packages/longhorn-poodle-svelte/src/settings/poodle/SettingsShell.svelte",
  );
  for (const token of [
    "const hasStagedUnit = $derived(",
    'unit.timing === "staged"',
    "{#if hasStagedUnit}",
  ]) {
    assertContains(shell, token, "Longhorn settings shell");
  }
  const tests = longhornRead("packages/longhorn-poodle-svelte/tests/settings-svelte/shell.test.ts");
  assertContains(
    tests,
    'queryByRole("button", { name: "Apply" })',
    "Longhorn immediate-page regression",
  );
}

function verifySoundcheckRegistryAndHost(): void {
  const workspace = soundcheckRead("Cargo.toml");
  const desktop = soundcheckRead("src-tauri/Cargo.toml");
  for (const dependency of fixture.packages.rust) {
    assertContains(
      workspace,
      `${dependency} = { path = "../longhorn/crates/${dependency}" }`,
      "Soundcheck Rust workspace dependency",
    );
    assertContains(
      desktop,
      `${dependency}.workspace = true`,
      "Soundcheck desktop dependency",
    );
  }
  const manifest = JSON.parse(soundcheckRead("package.json")) as {
    dependencies: Record<string, string>;
  };
  for (const dependency of fixture.packages.typescript) {
    assert(
      manifest.dependencies[dependency]?.startsWith("file:../longhorn/"),
      `Soundcheck TypeScript dependency missing: ${dependency}`,
    );
  }

  const settings = soundcheckRead("src-tauri/src/settings_host.rs");
  for (const token of [
    `const MODULE_ID: &str = "${fixture.registry.module}"`,
    `const SECTION_ID: &str = "${fixture.registry.section}"`,
    `pub(crate) const APP_SCOPE_ID: &str = "${fixture.registry.scope}"`,
    `pub(crate) const APP_APPLY_UNIT_ID: &str = "${fixture.registry.apply_unit}"`,
    "SettingsMutationTiming::Immediate",
    "reset_supported: false",
    "ConfigSettingsApplyUnit::new",
    "register_config_operations_settings",
  ]) {
    assertContains(settings, token, "Soundcheck settings host");
  }
  for (const page of fixture.registry.product_pages) {
    assertContains(settings, `"${page}"`, "Soundcheck product page");
  }
  for (const capability of fixture.registry.capabilities) {
    const constant = capabilityConstant(capability);
    assertContains(settings, `capability_id(${constant})`, "sealed capability");
  }

  const operations = soundcheckRead("src-tauri/src/config_operations.rs");
  for (const token of [
    `const DOMAIN_ID: &str = "${fixture.backup_restore.domain}"`,
    `const ADAPTER_ID: &str = "${fixture.backup_restore.adapter}"`,
    "ConfigOperationCapability::StorageDiagnostics",
    "ConfigOperationCapability::BackupInventory",
    "ConfigOperationCapability::RestoreInspection",
    "ConfigOperationCapability::RestoreAdapterExecution",
    "validate_sqlite(&backup.path)",
    "semantic_database_digest",
    "RestoreAdapterParticipationProjection::FailureAtomic",
    '"rolledBack"',
    '"recoveryRequired"',
    `LIBRARY_RESTORED_EVENT: &str = "${fixture.backup_restore.restore_event}"`,
  ]) {
    assertContains(operations, token, "Soundcheck config operations host");
  }
  for (const forbidden of [
    "ConfigOperationCapability::StorageTransition,",
    "ConfigOperationCapability::BackupCreate,",
    "ConfigOperationCapability::BackupExport,",
    "ConfigOperationCapability::BackupRetention,",
    "ConfigOperationCapability::RestoreExecution,",
    "ConfigOperationCapability::RestoreRecovery,",
  ]) {
    assertExcludes(operations, forbidden, "Soundcheck composed capability");
  }

  const app = soundcheckRead("src-tauri/src/lib.rs");
  for (const token of [
    "longhorn_tauri_settings::longhorn_settings_registry",
    "longhorn_tauri_settings::longhorn_settings_apply",
    "longhorn_tauri_config::longhorn_config_snapshot",
    "longhorn_tauri_config::longhorn_config_restore_inspect",
    "longhorn_tauri_config::longhorn_config_restore_adapter_execute",
  ]) {
    assertContains(app, token, "Soundcheck Tauri host");
  }
  for (const removed of [
    "app_settings::set_agent_review_settings",
    "library_commands::list_library_backups",
    "library_commands::restore_library_backup,",
  ]) {
    assertExcludes(app, removed, "superseded Tauri command");
  }
}

function verifyRendererCutover(): void {
  const dialog = soundcheckRead("src/components/SettingsDialog.svelte");
  for (const token of [
    'import { SettingsShell } from "@inflatable-cookie/longhorn-settings/poodle"',
    "new SettingsSession({",
    "new SettingsClient(transport)",
    "new ConfigOperationsClient(transport)",
    "<StorageSettingsPage",
    "<BackupSettingsPage",
    "<RestoreSettingsPage",
    `listen("${fixture.backup_restore.restore_event}"`,
  ]) {
    assertContains(dialog, token, "Soundcheck settings renderer");
  }
  for (const removed of [
    "<Dialog",
    "<SidebarNav",
    "pendingSave",
    "flushSettingsSaves",
  ]) {
    assertExcludes(dialog, removed, "superseded settings controller");
  }
  const agent = soundcheckRead(
    "src/components/settings/AgentReviewSettings.svelte",
  );
  assertContains(agent, "context.change(applyUnitId", "checked agent mutation");
  assertContains(
    agent,
    "value: { model: nextModel, reasoningEffort: nextReasoning }",
    "atomic agent intent",
  );
  assert(
    !soundcheckPathExists("src/components/BackupsSettings.svelte"),
    "renderer-owned backup component still exists",
  );
  const api = soundcheckRead("src/lib/api.ts");
  for (const removed of [
    "setAgentReviewSettings",
    "listLibraryBackups",
    "restoreLibraryBackup",
  ]) {
    assertExcludes(api, removed, "superseded renderer API");
  }
}

function capabilityConstant(capability: string): string {
  const values: Record<string, string> = {
    "longhorn:config.storage-diagnostics": "STORAGE_DIAGNOSTICS_CAPABILITY_ID",
    "longhorn:config.backup-inventory": "BACKUP_INVENTORY_CAPABILITY_ID",
    "longhorn:config.restore-inspection": "RESTORE_INSPECTION_CAPABILITY_ID",
    "longhorn:config.restore-adapter-execution":
      "RESTORE_ADAPTER_EXECUTION_CAPABILITY_ID",
  };
  const value = values[capability];
  if (value === undefined) throw new Error(`unknown capability ${capability}`);
  return value;
}

function longhornRead(path: string): string {
  return git(longhornRoot, ["show", `${longhornCommit}:${path}`]);
}

function soundcheckRead(path: string): string {
  return git(soundcheckRoot, ["show", `${soundcheckCommit}:${path}`]);
}

function soundcheckReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${soundcheckCommit}:${path}`], {
    cwd: soundcheckRoot,
  });
}

function soundcheckPathExists(path: string): boolean {
  return (
    git(soundcheckRoot, [
      "ls-tree",
      "--name-only",
      soundcheckCommit,
      path,
    ]).trim() === path
  );
}

function git(root: string, args: string[]): string {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" });
}

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

function assert(condition: boolean, label: string): asserts condition {
  if (!condition) throw new Error(`verification failed: ${label}`);
}

function assertContains(value: string, token: string, label: string): void {
  assert(value.includes(token), `${label} missing ${JSON.stringify(token)}`);
}

function assertExcludes(value: string, token: string, label: string): void {
  assert(!value.includes(token), `${label} retained ${JSON.stringify(token)}`);
}

function assertEqual(
  actual: string | number,
  expected: string | number,
  label: string,
): void {
  assert(actual === expected, `${label}: expected ${expected}, got ${actual}`);
}
