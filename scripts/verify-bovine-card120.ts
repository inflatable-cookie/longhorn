import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  identity: {
    canonical_application_id: string;
    profile: string;
    stable_storage_name: string | null;
    effective_leaf: string;
  };
  domain: Record<string, string | number>;
  legacy_import: Record<string, string | boolean>;
  settings: {
    host: string;
    pages: string[];
    capabilities: string[];
    apply_units: string[];
    timing: string;
    editable_fields: string[];
    read_only_fields: string[];
  };
  composition: {
    typescript_packages: string[];
    rust_packages: string[];
    forbidden_systems: string[];
    poodle_public_packages: number;
    source_aliases: boolean;
    package_manager_publication: boolean;
  };
  authority: Record<string, string[]>;
  validation: {
    rust_tests: number;
    svelte_errors: number;
    svelte_warnings: number;
    native_debug_bundle: string;
    private_artifact_fixture: string;
    unrelated_dirty_paths: number;
    live_storage_mutated: boolean;
  };
};

const longhornRoot = resolve(import.meta.dir, "..");
const bovineRoot = resolve(
  process.env.BOVINE_REPO ??
    resolve(longhornRoot, "../acowtancy/bovine-accelerator-desktop"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/bovine-card120/config-settings-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifySources();
verifyScopedCutover();
verifyIdentityAndDomain();
verifyMigration();
verifySettings();
verifyComposition();
const artifacts = verifyPrivateArtifacts();

console.log(
  JSON.stringify(
    {
      schema: "longhorn.bovine-config-settings-cutover-verification.v1",
      outcome: "pass",
      source: fixture.sources.bovine_cutover_commit,
      identity: fixture.identity,
      domain: fixture.domain.id,
      settings: fixture.settings,
      composition: {
        rust: fixture.composition.rust_packages.length,
        typescript: fixture.composition.typescript_packages.length,
        forbiddenSystemsAbsent: fixture.composition.forbidden_systems,
      },
      artifacts,
      authority: fixture.authority,
      liveStorageMutated: fixture.validation.live_storage_mutated,
      packageManagerPublication:
        fixture.composition.package_manager_publication,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  equal(fixture.schema, "longhorn.bovine-config-settings-cutover.v1", "fixture schema");
  equal(fixture.outcome, "pass", "fixture outcome");
  equal(fixture.identity.profile, "platform-native-v1", "storage profile");
  equal(fixture.identity.stable_storage_name, null, "stable storage override");
  equal(
    fixture.identity.effective_leaf,
    fixture.identity.canonical_application_id,
    "effective storage leaf",
  );
  equal(fixture.settings.pages, ["bovine:workspace", "longhorn:storage"], "settings pages");
  equal(fixture.settings.capabilities, ["storageDiagnostics"], "settings capabilities");
  equal(fixture.settings.apply_units, ["bovine:workspace.apply"], "settings apply units");
  assert(!fixture.composition.source_aliases, "source aliases claimed");
  assert(!fixture.composition.package_manager_publication, "package publication claimed");
  assert(!fixture.legacy_import.cleanup_authorized, "legacy cleanup authorized");
  assert(!fixture.legacy_import.dual_write, "dual-write rollback claimed");
  assert(!fixture.legacy_import.silent_fallback, "silent fallback claimed");
  assert(!fixture.validation.live_storage_mutated, "live storage mutation claimed");
}

function verifySources(): void {
  equal(
    git(bovineRoot, ["rev-parse", "HEAD"]),
    fixture.sources.bovine_cutover_commit,
    "Bovine HEAD",
  );
  for (const [root, commit] of [
    [longhornRoot, fixture.sources.longhorn_selected_source_commit],
    [bovineRoot, fixture.sources.bovine_prior_commit],
    [bovineRoot, fixture.sources.bovine_cutover_commit],
  ] as const) {
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  git(bovineRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.bovine_prior_commit,
    fixture.sources.bovine_cutover_commit,
  ]);
  equal(
    sha256(bovineReadRaw("src-tauri/Cargo.lock")),
    fixture.sources.bovine_cargo_lock_sha256,
    "Bovine Cargo lock",
  );
  equal(
    sha256(bovineReadRaw("bun.lock")),
    fixture.sources.bovine_bun_lock_sha256,
    "Bovine Bun lock",
  );

  const status = command(bovineRoot, ["git", "status", "--porcelain"])
    .trimEnd()
    .split("\n")
    .filter(Boolean);
  equal(status.length, fixture.validation.unrelated_dirty_paths, "unrelated dirty path count");
  assert(
    status.every((line) => /^( M|\?\?) (CHANGELOG\.md|docs\/)/.test(line)),
    `Bovine has dirty migration-owned paths:\n${status.join("\n")}`,
  );
}

function verifyScopedCutover(): void {
  const expected = [
    "bun.lock",
    "package.json",
    "src-tauri/Cargo.lock",
    "src-tauri/Cargo.toml",
    "src-tauri/src/config_operations.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/src/settings_host.rs",
    "src-tauri/src/storage_profile.rs",
    "src-tauri/src/workspace.rs",
    "src-tauri/src/workspace/domain.rs",
    "src-tauri/src/workspace/migration.rs",
    "src-tauri/src/workspace/mod.rs",
    "src-tauri/src/workspace/tests.rs",
    "src/App.svelte",
    "src/components/SettingsDialog.svelte",
    "src/components/WorkspaceSettingsPage.svelte",
    "src/main.ts",
    "tsconfig.json",
    "vite.config.ts",
  ];
  const changed = git(bovineRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.bovine_prior_commit}..${fixture.sources.bovine_cutover_commit}`,
  ]).split("\n").filter(Boolean).sort();
  equal(changed, expected, "cutover path inventory");
}

function verifyIdentityAndDomain(): void {
  const tauri = JSON.parse(bovineRead("src-tauri/tauri.conf.json"));
  equal(tauri.identifier, fixture.identity.canonical_application_id, "Tauri identifier");
  const profile = bovineRead("src-tauri/src/storage_profile.rs");
  for (const token of [
    'CANONICAL_APPLICATION_ID: &str = "com.acowtancy.bovine-accelerator"',
    "StorageProfileSelection::platform_native()",
    "StorageProfile::PlatformNativeV1",
    "stable_storage_name().is_some()",
    'layout.effective_leaf() != CANONICAL_APPLICATION_ID',
  ]) contains(profile, token, "storage profile");

  const domain = bovineRead("src-tauri/src/workspace/domain.rs");
  for (const token of [
    'DOMAIN_ID: &str = "bovine.workspace-preferences"',
    'DOMAIN_FILE: &str = "workspace-preferences.json"',
    "StorageClass::UserConfig",
    "SchemaVersion::new(SCHEMA_VERSION)",
    "DurabilityRequirement::Durable",
    "Duration::from_secs(1)",
    "retained: BTreeMap<String, Value>",
  ]) contains(domain, token, "workspace preference domain");
  const workspace = bovineRead("src-tauri/src/workspace/mod.rs");
  for (const token of [
    ".mutate(runtime.domain(), runtime.options()",
    "import_legacy_preferences",
  ]) contains(workspace, token, "workspace preference runtime");
}

function verifyMigration(): void {
  const migration = bovineRead("src-tauri/src/workspace/migration.rs");
  for (const token of [
    'BACKUP_FILE: &str = "bovine-workspace-card120-v1.json"',
    'RECEIPT_FILE: &str = "bovine-workspace-card120-v1.receipt.json"',
    'migration: "bovine-workspace-preferences-card120-v1"',
    "publish_verified_backup",
    "verify_receipt",
    "source_sha256",
    "target_sha256",
    "source_retained: legacy_path.exists()",
    "cleanup_authorized: false",
    "target conflicts with retained legacy settings",
  ]) contains(migration, token, "legacy migration");
  const tests = bovineRead("src-tauri/src/workspace/tests.rs");
  for (const test of [
    "invalid_selection_does_not_replace_the_remembered_workspace",
    "failed_publication_leaves_the_last_valid_document_intact",
    "restart_loads_the_same_versioned_preference_authority",
    "interrupted_legacy_import_resumes_from_verified_backup",
    "completed_legacy_import_rejects_backup_drift",
    "legacy_import_backs_up_exact_bytes_and_preserves_unknown_fields",
  ]) contains(tests, `fn ${test}()`, "workspace migration tests");
}

function verifySettings(): void {
  const host = bovineRead("src-tauri/src/settings_host.rs");
  for (const token of [
    'WORKSPACE_APPLY_UNIT_ID: &str = "bovine:workspace.apply"',
    "SettingsMutationTiming::Staged",
    "SettingsEditability::ReadOnly",
    "SettingsEditability::Editable",
    "STORAGE_DIAGNOSTICS_CAPABILITY_ID",
    "registry_contains_only_workspace_and_storage_pages",
    "longhorn_settings_config::STORAGE_SETTINGS_PAGE_ID",
    "registry.apply_units().count(), 1",
  ]) contains(host, token, "settings authority");
  const operations = bovineRead("src-tauri/src/config_operations.rs");
  contains(operations, "vec![ConfigOperationCapability::StorageDiagnostics]", "config capability");
  for (const forbidden of ["BackupCreateOutcome::Created", "RestoreExecuteOutcome::Committed"])
    excludes(operations, forbidden, "unsupported config operation");

  const dialog = bovineRead("src/components/SettingsDialog.svelte");
  for (const token of [
    'import { SettingsShell } from "@longhorn/settings/poodle"',
    "StorageSettingsPage",
    "WorkspaceSettingsPage",
    '<SettingsShell',
  ]) contains(dialog, token, "settings dialog");
}

function verifyComposition(): void {
  const manifest = JSON.parse(bovineRead("package.json")) as {
    dependencies: Record<string, string>;
  };
  const selectedTs = Object.keys(manifest.dependencies)
    .filter((name) => name.startsWith("@longhorn/"))
    .sort();
  equal(selectedTs, [...fixture.composition.typescript_packages].sort(), "TypeScript package graph");
  for (const name of selectedTs) {
    assert(manifest.dependencies[name].startsWith("file:../../longhorn/"), `${name} is not source-linked`);
  }
  const poodle = Object.keys(manifest.dependencies).filter((name) => name.startsWith("@poodle/"));
  equal(poodle.length, fixture.composition.poodle_public_packages, "Poodle package graph");

  const cargo = bovineRead("src-tauri/Cargo.toml");
  const selectedRust = [...cargo.matchAll(/^(longhorn-[\w-]+)\s*=\s*\{/gm)]
    .map((match) => match[1])
    .sort();
  equal(selectedRust, [...fixture.composition.rust_packages].sort(), "Rust package graph");
  const tree = command(bovineRoot, [
    "cargo", "tree", "--manifest-path", "src-tauri/Cargo.toml", "--edges", "normal", "--prefix", "none",
  ]);
  for (const system of fixture.composition.forbidden_systems) {
    const rustName = `longhorn-${system} `;
    assert(!tree.includes(rustName), `Rust graph resolves ${rustName.trim()}`);
    assert(!manifest.dependencies[`@longhorn/${system}`], `renderer graph resolves @longhorn/${system}`);
  }
  const vite = bovineRead("vite.config.ts");
  const tsconfig = bovineRead("tsconfig.json");
  excludes(vite, "resolveLonghorn", "Vite source alias");
  excludes(tsconfig, '"paths"', "TypeScript source alias");
  assert(!tree.includes("longhorn-layout "), "SplitView introduced layout authority");
}

function verifyPrivateArtifacts(): Record<string, unknown> {
  const proof = JSON.parse(command(longhornRoot, [
    "bun",
    "scripts/verify-secondary-consumer-private-artifacts.ts",
  ], {
    SECONDARY_CONSUMER_ADMISSION_FIXTURE:
      fixture.validation.private_artifact_fixture,
  }));
  equal(proof.outcome, "pass", "private artifact proof");
  equal(proof.sources.consumers.bovine, fixture.sources.bovine_cutover_commit, "artifact Bovine source");
  assert(proof.audits.oneSvelteRuntimePerRenderer, "duplicate Svelte runtime");
  assert(proof.audits.isolatedProducedArtifactInstalls, "artifacts were not installed in isolation");
  assert(!proof.audits.packageManagerPublication, "artifact proof published packages");
  return {
    typescript: proof.artifacts.longhornTypescriptArtifactSet,
    rust: proof.artifacts.longhornRustArtifactSet,
    poodle: proof.artifacts.poodleArtifactSet,
    oneSvelteRuntime: true,
  };
}

function bovineRead(path: string): string {
  return bovineReadRaw(path).toString("utf8");
}

function bovineReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${fixture.sources.bovine_cutover_commit}:${path}`], {
    cwd: bovineRoot,
  });
}

function git(root: string, args: string[]): string {
  return command(root, ["git", ...args]).trim();
}

function command(root: string, args: string[], env: Record<string, string> = {}): string {
  const result = spawnSync(args[0], args.slice(1), {
    cwd: root,
    env: { ...process.env, ...env },
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(`${args.join(" ")} failed\n${result.stdout}\n${result.stderr}`);
  }
  return result.stdout;
}

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

function contains(source: string, token: string, label: string): void {
  assert(source.includes(token), `${label} missing ${token}`);
}

function excludes(source: string, token: string, label: string): void {
  assert(!source.includes(token), `${label} retains ${token}`);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
