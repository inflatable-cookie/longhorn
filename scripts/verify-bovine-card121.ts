import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  composition: {
    canonical_application_id: string;
    hierarchy: string[];
    typescript_packages: string[];
    rust_packages: string[];
    forbidden_systems: string[];
    settings_pages: string[];
    settings_capabilities: string[];
    poodle_public_packages: number;
    package_manager_publication: boolean;
  };
  matrices: Record<string, string[]>;
  native: Record<string, unknown>;
  rollback: Record<string, unknown>;
  authority: Record<string, string[]>;
  validation: {
    rust_tests: number;
    renderer_tests: number;
    svelte_errors: number;
    svelte_warnings: number;
    current_binary_sha256: string;
    private_artifact_fixture: string;
    unrelated_dirty_paths: number;
    authored_content_changed: boolean;
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
      "fixtures/migration/bovine-card121/conformance-closeout-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifySourcesAndScope();
verifyStorageAndRollback();
verifySettingsLifetime();
const composition = verifyCompositionAndAuthority();
verifyBovineValidation();
const artifacts = verifyPrivateArtifacts();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: fixture.outcome,
      source: fixture.sources.bovine_closeout_commit,
      composition,
      matrices: Object.fromEntries(
        Object.entries(fixture.matrices).map(([name, tests]) => [name, tests.length]),
      ),
      native: fixture.native,
      rollback: fixture.rollback,
      artifacts,
      authority: fixture.authority,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  equal(
    fixture.schema,
    "longhorn.bovine-minimal-composition-conformance-closeout.v1",
    "fixture schema",
  );
  equal(
    fixture.outcome,
    "pass-with-environment-unmet-native-gui",
    "fixture outcome",
  );
  equal(
    fixture.composition.canonical_application_id,
    "com.acowtancy.bovine-accelerator",
    "canonical application id",
  );
  equal(fixture.composition.hierarchy, [], "shared hierarchy");
  assert(
    !fixture.composition.package_manager_publication,
    "package-manager publication claimed",
  );
  assert(!fixture.rollback.dual_write, "rollback used dual writes");
  assert(!fixture.rollback.silent_fallback, "rollback used silent fallback");
  assert(!fixture.rollback.cleanup_authorized, "legacy cleanup was authorized");
  assert(
    fixture.rollback.previous_workspace_tests_passed === true &&
      fixture.rollback.previous_workspace_tests === 7 &&
      fixture.rollback.temporary_worktree_removed === true,
    "previous-source rollback suite is incomplete",
  );
  equal(fixture.native.source_sha256, fixture.native.backup_sha256, "exact backup");
  assert(fixture.native.first_launch_stable === true, "first launch was not stable");
  assert(fixture.native.restart_stable === true, "restart was not stable");
  assert(fixture.native.acceptance_root_debug_only === true, "acceptance root escaped debug");
  assert(fixture.native.unknown_fields_retained === true, "unknown fields were lost");
  assert(!fixture.native.live_storage_mutated, "live storage mutation claimed");
  assert(!fixture.validation.authored_content_changed, "authored content change claimed");
}

function verifySourcesAndScope(): void {
  equal(
    git(bovineRoot, ["rev-parse", "HEAD"]),
    fixture.sources.bovine_closeout_commit,
    "Bovine HEAD",
  );
  for (const commit of [
    fixture.sources.longhorn_selected_source_commit,
    fixture.sources.bovine_previous_authority_commit,
    fixture.sources.bovine_cutover_commit,
    fixture.sources.bovine_closeout_commit,
  ]) {
    const root = commit === fixture.sources.longhorn_selected_source_commit
      ? longhornRoot
      : bovineRoot;
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  git(bovineRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.bovine_previous_authority_commit,
    fixture.sources.bovine_closeout_commit,
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
  equal(
    status.length,
    fixture.validation.unrelated_dirty_paths,
    "unrelated dirty path count",
  );
  assert(
    status.every((line) => /^( M|\?\?) (CHANGELOG\.md|docs\/)/.test(line)),
    `Bovine has dirty migration-owned paths:\n${status.join("\n")}`,
  );

  const expected = [
    "bun.lock",
    "effigy.toml",
    "package.json",
    "src-tauri/src/storage_profile.rs",
    "src-tauri/src/workspace/migration.rs",
    "src-tauri/src/workspace/tests.rs",
    "src/components/SettingsDialog.svelte",
    "src/components/SettingsDialog.test.ts",
    "src/components/SettingsDialogHarness.svelte",
    "src/components/settings-test-support.ts",
    "src/test-setup.ts",
    "vitest.config.ts",
  ];
  const changed = git(bovineRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.bovine_cutover_commit}..${fixture.sources.bovine_closeout_commit}`,
  ]).split("\n").filter(Boolean).sort();
  equal(changed, expected, "Card 121 path inventory");
}

function verifyStorageAndRollback(): void {
  const profile = bovineRead("src-tauri/src/storage_profile.rs");
  for (const token of [
    'ACCEPTANCE_ROOT_ENV: &str = "BOVINE_ACCEPTANCE_ROOT"',
    "#[cfg(debug_assertions)]",
    "resolve_acceptance_root",
    "must be absolute",
    "escaped its proof root",
    "acceptance_profile_is_explicit_and_confined",
  ]) contains(profile, token, "acceptance storage profile");

  const migration = bovineRead("src-tauri/src/workspace/migration.rs");
  for (const token of [
    "LoadOutcome::Ready(loaded) if loaded.source.is_some()",
    "Advanced workspace settings require recovery",
    "source != backup",
    "cleanup_authorized: false",
  ]) contains(migration, token, "migration restart authority");

  const tests = bovineRead("src-tauri/src/workspace/tests.rs");
  for (const test of fixture.matrices.storage.filter((name) => !name.startsWith("acceptance_"))) {
    contains(tests, `fn ${test}()`, "storage matrix");
  }

  const previous = git(bovineRoot, [
    "show",
    `${fixture.sources.bovine_previous_authority_commit}:src-tauri/src/workspace.rs`,
  ]);
  for (const field of [
    "workspace_root: Option<PathBuf>",
    "navigation_ratio: Option<f64>",
    "expanded_node_ids: Vec<String>",
    "selected_node_id: Option<String>",
  ]) contains(previous, field, "previous preference reader");
  excludes(previous, "deny_unknown_fields", "previous preference reader");
  contains(previous, "fn presentation_save_is_bounded_and_preserves_workspace()", "rollback suite");
}

function verifySettingsLifetime(): void {
  const dialog = bovineRead("src/components/SettingsDialog.svelte");
  for (const token of [
    "export let transport: EventTransport",
    "onDestroy(() =>",
    "void session.stop()",
    "{#if open}",
    "onOpenChange={handleOpenChange}",
  ]) contains(dialog, token, "settings lifetime");
  const test = bovineRead("src/components/SettingsDialog.test.ts");
  for (const token of [
    fixture.matrices.renderer[0],
    'name: "Workspace navigation width"',
    'name: /Wide/',
    "transport.ratio).toBe(0.32)",
    "transport.activeListenerCount()).toBe(0)",
    'name: "Reopen settings"',
  ]) contains(test, token, "renderer conformance");
  const effigy = bovineRead("effigy.toml");
  contains(effigy, '"check:renderer" = "bun run test"', "Bovine QA");
  contains(effigy, '{ task = "check:renderer" }', "Bovine QA");
}

function verifyCompositionAndAuthority(): Record<string, unknown> {
  const manifest = JSON.parse(bovineRead("package.json")) as {
    dependencies: Record<string, string>;
  };
  const selectedTs = Object.keys(manifest.dependencies)
    .filter((name) => name.startsWith("@longhorn/"))
    .sort();
  equal(selectedTs, [...fixture.composition.typescript_packages].sort(), "TypeScript graph");
  const poodle = Object.keys(manifest.dependencies).filter((name) => name.startsWith("@poodle/"));
  equal(poodle.length, fixture.composition.poodle_public_packages, "Poodle graph");

  const cargo = bovineRead("src-tauri/Cargo.toml");
  const selectedRust = [...cargo.matchAll(/^(longhorn-[\w-]+)\s*=\s*\{/gm)]
    .map((match) => match[1])
    .sort();
  equal(selectedRust, [...fixture.composition.rust_packages].sort(), "Rust graph");
  const tree = command(bovineRoot, [
    "cargo", "tree", "--manifest-path", "src-tauri/Cargo.toml", "--edges", "normal", "--prefix", "none",
  ]);
  for (const system of fixture.composition.forbidden_systems) {
    assert(!manifest.dependencies[`@longhorn/${system}`], `renderer resolves ${system}`);
    assert(!tree.includes(`longhorn-${system} `), `Rust graph resolves ${system}`);
  }
  assert(!tree.includes("longhorn-layout "), "SplitView became a layout document");

  const allRendererSources = command(bovineRoot, [
    "git", "grep", "-n", "@poodle/", fixture.sources.bovine_closeout_commit, "--", "src",
  ]);
  excludes(allRendererSources, "/src/", "private Poodle source import");
  const currentWorkspace = [
    bovineRead("src-tauri/src/workspace/mod.rs"),
    bovineRead("src-tauri/src/workspace/domain.rs"),
    bovineRead("src-tauri/src/workspace/migration.rs"),
  ].join("\n");
  for (const removed of ["PREFERENCES_FILE", "fn write_preferences", "fs::write(config_dir.join"])
    excludes(currentWorkspace, removed, "superseded generic writer");

  const retainedAuthorityDiff = git(bovineRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.bovine_previous_authority_commit}..${fixture.sources.bovine_closeout_commit}`,
    "--",
    "src-tauri/src/repository.rs",
    "src-tauri/src/git.rs",
    "src-tauri/src/search.rs",
    "src-tauri/src/editorial.rs",
    "src-tauri/src/model.rs",
    "src/lib/navigation.ts",
  ]);
  equal(retainedAuthorityDiff, "", "retained product authority diff");

  return {
    rustPackages: selectedRust.length,
    typescriptPackages: selectedTs.length,
    poodlePackages: poodle.length,
    hierarchy: fixture.composition.hierarchy,
    settingsPages: fixture.composition.settings_pages,
    settingsCapabilities: fixture.composition.settings_capabilities,
    forbiddenSystemsAbsent: fixture.composition.forbidden_systems,
    genericPreferenceAuthorities: 1,
  };
}

function verifyBovineValidation(): void {
  const output = command(bovineRoot, ["effigy", "qa"]);
  contains(output, "63 passed", "Bovine Rust validation");
  contains(output, "1 passed", "Bovine renderer validation");
  contains(output, "0 errors and 0 warnings", "Bovine Svelte validation");
}

function verifyPrivateArtifacts(): Record<string, unknown> {
  const proof = JSON.parse(command(
    longhornRoot,
    ["bun", "scripts/verify-secondary-consumer-private-artifacts.ts"],
    { SECONDARY_CONSUMER_ADMISSION_FIXTURE: fixture.validation.private_artifact_fixture },
  ));
  equal(proof.outcome, "pass", "private artifact proof");
  equal(
    proof.sources.consumers.bovine,
    fixture.sources.bovine_closeout_commit,
    "artifact Bovine source",
  );
  equal(proof.writeAdmission.nextCard, 122, "next artifact admission");
  assert(proof.audits.oneSvelteRuntimePerRenderer, "duplicate Svelte runtime");
  assert(proof.audits.isolatedProducedArtifactInstalls, "artifact installs were not isolated");
  assert(!proof.audits.packageManagerPublication, "artifact proof published packages");
  return {
    typescript: proof.artifacts.longhornTypescriptArtifactSet,
    rust: proof.artifacts.longhornRustArtifactSet,
    poodle: proof.artifacts.poodleArtifactSet,
    oneSvelteRuntime: true,
    nextCard: 122,
  };
}

function bovineRead(path: string): string {
  return bovineReadRaw(path).toString("utf8");
}

function bovineReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${fixture.sources.bovine_closeout_commit}:${path}`], {
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
  const left = JSON.stringify(actual);
  const right = JSON.stringify(expected);
  assert(left === right, `${label}: expected ${right}, got ${left}`);
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
