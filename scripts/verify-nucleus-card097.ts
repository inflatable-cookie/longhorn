import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const nucleusRoot = resolve(
  process.env.NUCLEUS_REPO ?? resolve(longhornRoot, "../nucleus"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/nucleus-card097/window-cutover-v1.json",
    ),
    "utf8",
  ),
) as WindowCutoverFixture;
const nucleusCommit = fixture.sources.nucleus_cutover_commit;
const longhornCommit = fixture.sources.longhorn_window_host_commit;

git(nucleusRoot, ["cat-file", "-e", `${nucleusCommit}^{commit}`]);
git(longhornRoot, ["cat-file", "-e", `${longhornCommit}^{commit}`]);
git(nucleusRoot, [
  "merge-base",
  "--is-ancestor",
  fixture.sources.nucleus_prior_commit,
  nucleusCommit,
]);

verifyFixture();
verifyNucleusCommit();
verifyLonghornCommit();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: "pass",
      nucleusCommit,
      longhornCommit,
      logicalWindow: fixture.identity.logical_window_id,
      transportLabel: fixture.identity.tauri_label,
      placementDomain: fixture.storage.domain,
      fallbackOrder: fixture.placement.fallback_order,
      nativeProof: fixture.native_proof,
      surfacePackages: false,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.nucleus-protected-window-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(fixture.identity.logical_window_id, "window:primary", "logical id");
  assertEqual(fixture.identity.tauri_label, "main", "Tauri label");
  assert(fixture.identity.protected_primary, "primary is not protected");
  assert(!fixture.identity.dynamic_factory, "Nucleus gained a dynamic factory");
  assertEqual(fixture.storage.domain, "nucleus.window-placement", "domain id");
  assertEqual(fixture.storage.storage_class, "machine-state", "storage class");
  assert(fixture.storage.backup_verified_before_source_removal, "backup ordering");
  assert(fixture.storage.interrupted_receipt_completion, "receipt recovery");
  assert(!fixture.storage.renderer_write_authority, "renderer placement authority");
  assert(!fixture.storage.dual_write, "window dual write");
  assert(!fixture.storage.silent_legacy_fallback, "legacy fallback");
  assertEqual(fixture.placement.fallback_order.length, 4, "fallback count");
  assertEqual(fixture.lifecycle.reveal_signals.length, 2, "reveal signal count");
  assertEqual(fixture.lifecycle.flush_timeout_ms, 1000, "flush timeout");
  assertEqual(fixture.native_proof.fresh_start, "converged-attempt-1", "fresh start");
  assertEqual(fixture.native_proof.restart, "converged-attempt-1", "restart");
  assertEqual(
    fixture.native_proof.tauri_empty_available_with_valid_primary,
    "admit-primary-as-sole-observation",
    "empty available monitor policy",
  );
  assert(!fixture.capability.wildcard, "wildcard capability");
  assert(!fixture.capability.remote_child_authority, "remote child authority");
  assert(!fixture.packages.surface_packages, "Surface package");
  assert(!fixture.packages.package_manager_publication, "package publication");
}

function verifyNucleusCommit(): void {
  const cargo = nucleusRead("apps/desktop/src-tauri/Cargo.toml");
  for (const dependency of fixture.packages.required) {
    assertContains(cargo, dependency, "Nucleus desktop dependency");
  }
  assertExcludes(cargo, "longhorn-surface", "Nucleus desktop dependency");

  assertEqual(
    createHash("sha256").update(nucleusReadRaw("Cargo.lock")).digest("hex"),
    fixture.sources.nucleus_lock_sha256,
    "Nucleus lockfile digest",
  );

  const host = nucleusRead("apps/desktop/src-tauri/src/window_host/mod.rs");
  for (const token of [
    'PRIMARY_WINDOW_ID: &str = "window:primary"',
    'MAIN_LABEL: &str = "main"',
    "assemble_tauri_window_host",
    "ProtectedPrimaryPolicy::Preserve",
    "NoWindowFactory",
    "UniformScaleMapper",
    "restore_window_placement",
    ".for_hidden_restore()",
    "TauriAsyncWindowLifecycleScheduler",
    "ConfigWindowPlacementSink",
    "MAX_RESTORE_ATTEMPTS: u8 = 8",
    "close_app.exit(0)",
  ]) {
    assertContains(host, token, "Nucleus window host");
  }

  const domain = nucleusRead("apps/desktop/src-tauri/src/window_host/domain.rs");
  for (const token of [
    'DOMAIN_ID: &str = "nucleus.window-placement"',
    "StorageClass::MachineState",
    "KnownDisplayRegistry",
    "SavedWindowPlacement",
  ]) {
    assertContains(domain, token, "Nucleus window domain");
  }

  const migration = nucleusRead("apps/desktop/src-tauri/src/window_host/migration.rs");
  for (const token of [
    "publish_verified_backup",
    "Sha256Digest::from_bytes",
    "fs::remove_file(target_path)",
    "published_domain_without_receipt_resumes_receipt_completion_only",
    "SavedDisplayAssociation::unresolved()",
  ]) {
    assertContains(migration, token, "Nucleus placement migration");
  }

  const app = nucleusRead("apps/desktop/src/App.svelte");
  assertContains(app, 'invoke("desktop_window_page_ready")', "renderer ready signal");
  const desktop = nucleusRead("apps/desktop/src-tauri/src/lib.rs");
  assertContains(desktop, "window_host::install", "desktop host install");
  assertContains(desktop, "window_host::teardown", "desktop host teardown");
  assertContains(desktop, "desktop_window_page_ready", "page-ready command");
  assertExcludes(desktop, "window_geometry", "retired window module");
  assert(!nucleusFileExists("apps/desktop/src-tauri/src/window_geometry.rs"), "old window geometry file remains");

  const workspace = nucleusRead("apps/desktop/src-tauri/src/workspace_ui.rs");
  assertExcludes(workspace, "update_workspace_window_placement", "renderer placement writer");
  assertExcludes(workspace, "load_workspace_window_placement", "duplicate placement reader");

  const tauri = JSON.parse(
    nucleusRead("apps/desktop/src-tauri/tauri.conf.json"),
  ) as {
    identifier: string;
    app: {
      windows: Array<Record<string, unknown>>;
      security: { capabilities: Array<{ webviews: string[] }> };
    };
  };
  assertEqual(tauri.identifier, "com.inflatablecookie.nucleus", "application id");
  const main = tauri.app.windows[0];
  assertEqual(main?.label, "main", "main window label");
  assertEqual(main?.visible, false, "hidden startup");
  assertEqual(main?.width, 1280, "default width");
  assertEqual(main?.height, 820, "default height");
  assertEqual(main?.minWidth, 900, "minimum width");
  assertEqual(main?.minHeight, 620, "minimum height");
  assertEqual(main?.titleBarStyle, "Overlay", "titlebar style");
  assertEqual(
    JSON.stringify(tauri.app.security.capabilities[0]?.webviews),
    JSON.stringify(["main"]),
    "trusted webviews",
  );
  assertEqual(
    nucleusRead("apps/desktop/src-tauri/tauri.conf.json"),
    git(nucleusRoot, [
      "show",
      `${fixture.sources.nucleus_prior_commit}:apps/desktop/src-tauri/tauri.conf.json`,
    ]),
    "capability and retained window configuration",
  );
}

function verifyLonghornCommit(): void {
  const probe = longhornRead("crates/longhorn-tauri-windowing/src/probe.rs");
  assertContains(probe, "SoleObservedPrimary", "Tauri display probe");
  assertContains(probe, "available.push(primary.clone())", "sole-primary admission");
  assertExcludes(probe, "work_position", "primary identity key");
  assertExcludes(probe, "work_size", "primary identity key");

  const probeTests = longhornRead(
    "crates/longhorn-tauri-windowing/src/probe/tests.rs",
  );
  assertContains(
    probeTests,
    "valid_primary_becomes_the_sole_observation_when_available_is_empty",
    "sole-primary regression",
  );

  const composition = longhornRead(
    "crates/longhorn-tauri-windowing/tests/composition/fixtures.rs",
  );
  assertContains(
    composition,
    "nucleus_single_window_restore_shutdown_and_repeated_init_share_one_host",
    "Nucleus host conformance",
  );
  const lifecycle = longhornRead(
    "crates/longhorn-tauri-windowing/tests/lifecycle/events.rs",
  );
  assertContains(
    lifecycle,
    "nucleus_settles_captures_blur_and_uses_one_second_close_bound",
    "Nucleus lifecycle conformance",
  );
  assertContains(
    lifecycle,
    "loophole_programmatic_geometry_is_suppressed_before_sink_mutation",
    "programmatic attribution conformance",
  );
}

function nucleusRead(path: string): string {
  return git(nucleusRoot, ["show", `${nucleusCommit}:${path}`]);
}

function nucleusReadRaw(path: string): string {
  return execFileSync("git", ["show", `${nucleusCommit}:${path}`], {
    cwd: nucleusRoot,
    encoding: "utf8",
  });
}

function longhornRead(path: string): string {
  return git(longhornRoot, ["show", `${longhornCommit}:${path}`]);
}

function nucleusFileExists(path: string): boolean {
  return git(nucleusRoot, ["ls-tree", "-r", "--name-only", nucleusCommit, "--", path]) === path;
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

interface WindowCutoverFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_window_host_commit: string;
    readonly nucleus_prior_commit: string;
    readonly nucleus_cutover_commit: string;
    readonly nucleus_lock_sha256: string;
  };
  readonly identity: {
    readonly logical_window_id: string;
    readonly tauri_label: string;
    readonly protected_primary: boolean;
    readonly dynamic_factory: boolean;
  };
  readonly storage: {
    readonly domain: string;
    readonly storage_class: string;
    readonly backup_verified_before_source_removal: boolean;
    readonly interrupted_receipt_completion: boolean;
    readonly renderer_write_authority: boolean;
    readonly dual_write: boolean;
    readonly silent_legacy_fallback: boolean;
  };
  readonly placement: { readonly fallback_order: readonly string[] };
  readonly lifecycle: {
    readonly reveal_signals: readonly string[];
    readonly flush_timeout_ms: number;
  };
  readonly native_proof: {
    readonly fresh_start: string;
    readonly restart: string;
    readonly tauri_empty_available_with_valid_primary: string;
  };
  readonly capability: {
    readonly wildcard: boolean;
    readonly remote_child_authority: boolean;
  };
  readonly packages: {
    readonly required: readonly string[];
    readonly surface_packages: boolean;
    readonly package_manager_publication: boolean;
  };
}
