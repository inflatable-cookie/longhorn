import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  donor_trace: Record<string, boolean | number | string | string[]>;
  operation: Record<string, boolean | number | string>;
  ownership: Record<string, boolean | string[]>;
  renderer: Record<string, boolean>;
  composition: {
    rust: string[];
    typescript: string[];
    bridge_dependency: boolean;
    notification_dependency: boolean;
    durable_scheduler_dependency: boolean;
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
      "fixtures/migration/soundcheck-card117/plugin-scan-operation-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;
const soundcheckCommit = fixture.sources.soundcheck_cutover_commit;

verifyReceipts();
verifyFixture();
verifyLonghornBoundary();
verifySoundcheckHost();
verifyRenderer();
verifyRetainedDonor();

console.log(JSON.stringify({
  schema: "longhorn.soundcheck-plugin-scan-operation-verification.v1",
  outcome: "pass",
  soundcheckCommit,
  operationKind: fixture.operation.kind,
  operationScope: fixture.operation.scope,
  retainedAuthority: fixture.donor_trace.authority,
  nativeGuiProof: fixture.validation.native_scan_gui_proof,
  packageManagerPublication: fixture.composition.package_manager_publication,
}, null, 2));

function verifyReceipts(): void {
  for (const [root, commit] of [
    [longhornRoot, fixture.sources.longhorn_source_commit],
    [soundcheckRoot, soundcheckCommit],
    [libraryRoot, fixture.sources.soundcheck_library_commit],
  ] as const) {
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  git(soundcheckRoot, [
    "merge-base", "--is-ancestor",
    fixture.sources.soundcheck_prior_commit,
    soundcheckCommit,
  ]);
  git(libraryRoot, [
    "merge-base", "--is-ancestor",
    fixture.sources.soundcheck_library_commit,
    "HEAD",
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
}

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.soundcheck-plugin-scan-operation-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assert(fixture.donor_trace.same_active_reuse === true, "same-active reuse");
  assert(fixture.operation.terminal_immutable === true, "terminal immutability");
  assert(
    fixture.operation.cancellation_dispatch_identity_checked === true,
    "cancellation identity",
  );
  assert(
    fixture.ownership.product_payload_in_operation_protocol === false,
    "product payload entered operation protocol",
  );
  assert(fixture.renderer.teardown_cancels_work === false, "teardown cancellation");
  assert(!fixture.composition.bridge_dependency, "bridge dependency admitted");
  assert(!fixture.composition.notification_dependency, "notification dependency admitted");
  assert(!fixture.composition.durable_scheduler_dependency, "scheduler dependency admitted");
  assert(!fixture.composition.package_manager_publication, "publication claimed");
  assert(fixture.validation.live_data_mutated === false, "live data mutation claimed");
}

function verifyLonghornBoundary(): void {
  const library = longhornRead("crates/longhorn-operation/src/lib.rs");
  for (const token of [
    "Consumers own admission, scheduling, execution,",
    "product progress, outcomes, artifacts, persistence, and recovery",
  ]) assertContains(library, token, "Longhorn operation authority boundary");
  const compatibility = longhornRead("packages/operation/src/compatibility.ts");
  for (const token of ['"payload"', '"result"', '"artifact"', '"report"', '"log"']) {
    assertContains(compatibility, token, "payload-free protocol guard");
  }
  const tauri = longhornRead("crates/longhorn-tauri-operation/src/commands.rs");
  for (const token of [
    "longhorn_operation_snapshot",
    "longhorn_operation_mutate",
    "longhorn_operation_cancel",
    'OPERATION_CHANGED_EVENT: &str = "longhorn://operation/changed"',
  ]) assertContains(tauri, token, "Longhorn Tauri operation host");
}

function verifySoundcheckHost(): void {
  const workspace = soundcheckRead("Cargo.toml");
  const desktop = soundcheckRead("src-tauri/Cargo.toml");
  for (const dependency of fixture.composition.rust) {
    assertContains(
      workspace,
      `${dependency} = { path = "../longhorn/crates/${dependency}" }`,
      "Soundcheck Rust workspace dependency",
    );
    assertContains(desktop, `${dependency}.workspace = true`, "Soundcheck Rust dependency");
  }
  for (const forbidden of ["longhorn-bridge", "longhorn-notifications"]) {
    assertExcludes(desktop, forbidden, "Soundcheck operation dependency");
  }
  const manifest = JSON.parse(soundcheckRead("package.json")) as {
    dependencies: Record<string, string>;
  };
  for (const dependency of fixture.composition.typescript) {
    assert(
      manifest.dependencies[dependency]?.startsWith("file:../longhorn/"),
      `Soundcheck TypeScript dependency missing: ${dependency}`,
    );
  }

  const host = soundcheckRead("src-tauri/src/operation_host.rs");
  for (const token of [
    'SCAN_OPERATION_KIND: &str = "soundcheck.plugin-scan"',
    'AUTHORITY_ID: &str = "soundcheck:operations"',
    'LIBRARY_SCOPE_ID: &str = "soundcheck:library"',
    'MAIN_CALLER: &str = "main"',
    "OperationCatalogueLimits::new(1, 16, 64 * 1_024)",
    "current_scan_snapshot(&self.app)",
    '.starts_with("retry:")',
    "OperationStateProjection::Cancelling",
    "current.job_id != operation_id.as_str()",
    '"staleOperation"',
    "publish_operation_changed(app, event)",
    "current.is_none_or(|value| candidate > value)",
  ]) assertContains(host, token, "Soundcheck operation host");
  for (const test of [
    "reconnect_registers_database_job_and_terminal_truth",
    "same_active_is_reused_and_cancelling_can_still_succeed",
    "retry_gets_new_identity_and_terminal_lineage",
  ]) assertContains(host, test, "Soundcheck operation regression");

  const commands = soundcheckRead("src-tauri/src/library_commands.rs");
  for (const token of [
    "start_plugin_scan",
    "plugin_scan_status",
    "retry_plugin_scan_unit",
    "reconcile_scan_operation",
  ]) assertContains(commands, token, "Soundcheck scan command");
  assertExcludes(commands, "pub(crate) fn cancel_plugin_scan", "direct cancellation command");

  const app = soundcheckRead("src-tauri/src/lib.rs");
  for (const token of [
    "longhorn_tauri_operation::longhorn_operation_snapshot",
    "longhorn_tauri_operation::longhorn_operation_mutate",
    "longhorn_tauri_operation::longhorn_operation_cancel",
    "SoundcheckScanExecutor::new",
  ]) assertContains(app, token, "Soundcheck operation assembly");
  assertExcludes(app, "library_commands::cancel_plugin_scan", "direct cancellation registration");
}

function verifyRenderer(): void {
  const operations = soundcheckRead("src/lib/scan-operations.ts");
  for (const token of [
    "createTauriOperationPort",
    "new OperationSession({ port })",
    "await client.snapshot()",
    "expectedOperationRevision: operation.revision",
  ]) assertContains(operations, token, "Soundcheck operation renderer");
  const app = soundcheckRead("src/App.svelte");
  for (const token of [
    "createScanOperationSession()",
    "await cancelActiveScanOperation()",
    "activeScanOperation(scanOperationSession.active)",
    "scanOperationSession.start()",
    "scanOperationSession.stop()",
  ]) assertContains(app, token, "Soundcheck renderer session");
  const cleanup = app.slice(app.indexOf("return () => {"), app.indexOf("</script>"));
  assertExcludes(cleanup, "cancelActiveScanOperation", "renderer teardown");
  const api = soundcheckRead("src/lib/api.ts");
  assertExcludes(api, 'invoke("cancel_plugin_scan")', "renderer direct cancel API");
}

function verifyRetainedDonor(): void {
  const jobs = libraryRead("crates/soundcheck-library-jobs/src/lib.rs");
  for (const token of [
    "acquire_plugin_scan_lease",
    "PluginScanOperationResult::Occupied",
    "request_plugin_scan_cancellation",
    "retry_plugin_inventory_scan_unit",
  ]) assertContains(jobs, token, "soundcheck-library job authority");
  const store = libraryRead("packages/library-client/src/store.ts");
  for (const token of [
    "setTimeout(() => void poll(), 500)",
    "scanJob: PluginScanSnapshot | null",
    "retryScanUnit",
    "diagnostics:",
  ]) assertContains(store, token, "soundcheck-library product projection");
}

function soundcheckRead(path: string): string {
  return git(soundcheckRoot, ["show", `${soundcheckCommit}:${path}`]);
}

function soundcheckReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${soundcheckCommit}:${path}`], {
    cwd: soundcheckRoot,
  });
}

function libraryRead(path: string): string {
  return git(libraryRoot, ["show", `${fixture.sources.soundcheck_library_commit}:${path}`]);
}

function longhornRead(path: string): string {
  return git(longhornRoot, ["show", `${fixture.sources.longhorn_source_commit}:${path}`]);
}

function git(root: string, args: string[]): string {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" });
}

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

function assertContains(value: string, token: string, label: string): void {
  assert(value.includes(token), `${label} is missing ${JSON.stringify(token)}`);
}

function assertExcludes(value: string, token: string, label: string): void {
  assert(!value.includes(token), `${label} unexpectedly contains ${JSON.stringify(token)}`);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  assert(actual === expected, `${label}: expected ${String(expected)}, got ${String(actual)}`);
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
