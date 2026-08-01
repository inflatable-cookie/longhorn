import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const nucleusRoot = resolve(
  process.env.NUCLEUS_REPO ?? resolve(repoRoot, "../nucleus"),
);
const fixturePath = resolve(
  repoRoot,
  "fixtures/migration/nucleus-card096/storage-cutover-v1.json",
);
const fixture = JSON.parse(readFileSync(fixturePath, "utf8")) as CutoverFixture;
const commit = fixture.sources.nucleus_cutover_commit;

git(nucleusRoot, ["cat-file", "-e", `${commit}^{commit}`]);
git(repoRoot, ["cat-file", "-e", `${fixture.sources.longhorn_commit}^{commit}`]);
git(nucleusRoot, [
  "merge-base",
  "--is-ancestor",
  fixture.sources.nucleus_prior_commit,
  commit,
]);
verifyFixture();
verifyConsumerCommit();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: "pass",
      nucleusCommit: commit,
      canonicalApplicationId: fixture.identity.canonical_application_id,
      profiles: {
        default: fixture.identity.default_profile,
        proof: fixture.identity.proof_profile,
      },
      platforms: fixture.path_matrix.map((entry) => entry.platform),
      migratedDomains: fixture.stores.map((store) => store.domain),
      scenarios: fixture.scenarios.map((scenario) => scenario.id),
      sourceRetained: fixture.authority.legacy_source_retained,
      locatorCommit: fixture.authority.locator_commit,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  assertEqual(fixture.schema, "longhorn.nucleus-storage-cutover.v1", "fixture schema");
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(
    fixture.identity.canonical_application_id,
    "com.inflatablecookie.nucleus",
    "canonical application id",
  );
  assertEqual(fixture.identity.stable_storage_name, null, "stable storage name");
  assertEqual(fixture.identity.default_profile, "platform-native-v1", "default profile");
  assertEqual(fixture.identity.proof_profile, "portable-v1", "proof profile");
  assertEqual(fixture.path_matrix.length, 3, "platform path count");
  for (const entry of fixture.path_matrix) {
    assert(
      entry.config.includes(fixture.identity.canonical_application_id),
      `${entry.platform} config path lacks canonical id`,
    );
    assert(
      entry.database.includes(fixture.identity.canonical_application_id),
      `${entry.platform} database path lacks canonical id`,
    );
    assert(
      entry.database.endsWith("/databases/nucleus.sqlite"),
      `${entry.platform} database path is not durable data`,
    );
  }
  assertEqual(fixture.stores.length, 4, "migrated domain count");
  assertEqual(fixture.authority.locator_commit, "last", "locator ordering");
  assert(fixture.authority.fixed_locator, "fixed locator is not required");
  assert(fixture.authority.legacy_source_retained, "legacy source is not retained");
  assert(fixture.authority.unknown_source_files_retained, "unknown files are not retained");
  assert(!fixture.authority.dual_write, "dual write remains enabled");
  assert(
    !fixture.authority.normal_startup_legacy_fallback,
    "normal startup retains legacy fallback",
  );
  for (const id of [
    "missing-legacy-root",
    "corrupt-or-future-ui",
    "occupied-target",
    "live-sqlite-wal-and-four-domains",
    "committed-startup-replay",
    "overlapping-roots",
    "interrupted-before-locator",
    "interrupted-after-locator",
  ]) {
    assert(
      fixture.scenarios.some((scenario) => scenario.id === id),
      `missing scenario ${id}`,
    );
  }
}

function verifyConsumerCommit(): void {
  const tauri = gitRead("apps/desktop/src-tauri/tauri.conf.json");
  assertContains(tauri, '"identifier": "com.inflatablecookie.nucleus"', "Tauri identifier");
  assertExcludes(tauri, "dev.nucleus.desktop", "old Tauri identifier");

  const cargo = gitRead("apps/desktop/src-tauri/Cargo.toml");
  for (const dependency of [
    "longhorn-config",
    "longhorn-core",
    "longhorn-tauri-config",
    'features = ["backup", "bundled"]',
  ]) {
    assertContains(cargo, dependency, "desktop Rust dependency");
  }

  assertEqual(
    createHash("sha256").update(gitReadRaw("Cargo.lock")).digest("hex"),
    fixture.sources.nucleus_lock_sha256,
    "Nucleus lockfile digest",
  );

  const profile = gitRead("apps/desktop/src-tauri/src/desktop_profile.rs");
  for (const token of [
    "com.inflatablecookie.nucleus",
    "StorageProfile::PortableV1",
    "inspect_storage_bootstrap",
    "platform_directory_facts",
    "NUCLEUS_DESKTOP_PORTABLE_ROOT",
    "durable_database_dir",
    "nucleus.sqlite",
    "StorageBootstrapOrigin::Locator",
    "read_import_receipt",
  ]) {
    assertContains(profile, token, "desktop profile");
  }
  assertExcludes(profile, "NUCLEUS_DESKTOP_DATA_ROOT", "retired proof root environment");

  const migration = gitRead("apps/desktop/src-tauri/src/storage_migration.rs");
  for (const token of [
    "execute_storage_transition",
    "recover_storage_transition",
    "BackupAdapter",
    "backup(MAIN_DB",
    "restore(MAIN_DB",
    "window-placement.json",
    "project-layouts.json",
    "legacy-import-receipt.json",
    "read_optional_payload",
    "StorageTransitionOutcome::TargetCommitted",
  ]) {
    assertContains(migration, token, "storage migration");
  }

  const workspace = gitRead("apps/desktop/src-tauri/src/workspace_ui.rs");
  for (const token of ["WorkspaceUiPaths", "window_placement", "project_layouts"]) {
    assertContains(workspace, token, "split workspace UI authority");
  }
  assertExcludes(workspace, "workspace_ui_config_path", "combined runtime UI path");

  const proof = gitRead("scripts/native-proof.sh");
  assertContains(proof, "NUCLEUS_DESKTOP_PORTABLE_ROOT", "native proof profile");
  assertExcludes(proof, "NUCLEUS_DESKTOP_DATA_ROOT", "retired native proof profile");

  const currentDesktop = git(nucleusRoot, [
    "grep",
    "-n",
    "dev.nucleus.desktop",
    commit,
    "--",
    "apps/desktop",
  ], true);
  assertEqual(currentDesktop, "", "old id in current desktop tree");
}

function gitRead(path: string): string {
  return git(nucleusRoot, ["show", `${commit}:${path}`]);
}

function gitReadRaw(path: string): string {
  return execFileSync("git", ["show", `${commit}:${path}`], {
    cwd: nucleusRoot,
    encoding: "utf8",
  });
}

function git(root: string, arguments_: string[], allowFailure = false): string {
  try {
    return execFileSync("git", arguments_, {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", allowFailure ? "ignore" : "pipe"],
    }).trim();
  } catch (error) {
    if (allowFailure) return "";
    throw error;
  }
}

function assertContains(value: string, token: string, label: string): void {
  assert(value.includes(token), `${label} lacks ${JSON.stringify(token)}`);
}

function assertExcludes(value: string, token: string, label: string): void {
  assert(!value.includes(token), `${label} retains ${JSON.stringify(token)}`);
}

function assert(condition: boolean, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (actual !== expected) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface CutoverFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_commit: string;
    readonly nucleus_prior_commit: string;
    readonly nucleus_cutover_commit: string;
    readonly nucleus_lock_sha256: string;
  };
  readonly identity: {
    readonly canonical_application_id: string;
    readonly stable_storage_name: string | null;
    readonly default_profile: string;
    readonly proof_profile: string;
    readonly proof_root_environment: string;
  };
  readonly path_matrix: readonly {
    readonly platform: string;
    readonly config: string;
    readonly database: string;
  }[];
  readonly stores: readonly {
    readonly domain: string;
    readonly source: string;
    readonly targets: readonly string[];
    readonly adapter: string;
  }[];
  readonly authority: {
    readonly fixed_locator: boolean;
    readonly locator_commit: string;
    readonly typed_import_receipt: string;
    readonly legacy_source_retained: boolean;
    readonly unknown_source_files_retained: boolean;
    readonly dual_write: boolean;
    readonly normal_startup_legacy_fallback: boolean;
    readonly committed_startup_replay: string;
    readonly cleanup_deferred: boolean;
  };
  readonly scenarios: readonly {
    readonly id: string;
    readonly result: string;
    readonly proof?: string;
  }[];
}
