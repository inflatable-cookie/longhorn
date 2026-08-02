import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  coordination: Record<string, boolean | number | string>;
  helper: Record<string, boolean | number | string>;
  ownership: Record<string, boolean | string[]>;
  renderer: Record<string, boolean>;
  platform: Record<string, boolean | string | string[]>;
  composition: {
    rust: string[];
    typescript: string[];
    surface_dependency: boolean;
    transfer_dependency: boolean;
    bridge_dependency: boolean;
    package_manager_publication: boolean;
  };
  validation: Record<string, boolean | number | string | string[]>;
};

const longhornRoot = resolve(import.meta.dir, "..");
const soundcheckRoot = resolve(
  process.env.SOUNDCHECK_REPO ?? resolve(longhornRoot, "../soundcheck"),
);
const signalRoot = resolve(
  process.env.SIGNAL_REPO ?? resolve(longhornRoot, "../signal"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/soundcheck-card118/isolated-window-coordination-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;
const soundcheckCommit = fixture.sources.soundcheck_cutover_commit;

verifyReceipts();
verifyFixture();
verifyLonghornBoundary();
verifySoundcheckCoordination();
verifyHelperLifecycle();
verifyRenderer();
verifyRetainedAuthorities();

console.log(JSON.stringify({
  schema: "longhorn.soundcheck-isolated-window-coordination-verification.v1",
  outcome: "pass",
  soundcheckCommit,
  signalCommit: fixture.sources.signal_commit,
  islandPrefix: fixture.coordination.island_prefix,
  nativeProof: fixture.validation.native_plugin_matrix,
  packageManagerPublication: fixture.composition.package_manager_publication,
}, null, 2));

function verifyReceipts(): void {
  for (const [root, commit] of [
    [longhornRoot, fixture.sources.longhorn_source_commit],
    [soundcheckRoot, soundcheckCommit],
    [signalRoot, fixture.sources.signal_commit],
  ] as const) git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  git(soundcheckRoot, [
    "merge-base", "--is-ancestor",
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
}

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.soundcheck-isolated-window-coordination-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  for (const key of [
    "generation_checked",
    "listener_before_launch",
    "initial_plugin_size_authoritative",
    "desired_observed_convergence",
    "resize_decisions_receipted",
    "constrained_resize_retained",
    "request_gap_terminal",
  ]) assert(fixture.coordination[key] === true, `coordination claim failed: ${key}`);
  assert(fixture.helper.crash_terminal === true, "crash is not terminal");
  assert(fixture.renderer.observation_only === true, "renderer is not observation-only");
  assert(fixture.renderer.mutation_surface_exposed === false, "renderer mutation exposed");
  assert(fixture.ownership.product_payload_in_longhorn === false, "product payload admitted");
  assert(fixture.ownership.raw_native_pointer_in_longhorn === false, "native pointer admitted");
  assert(fixture.ownership.plugin_unload_entered_longhorn === false, "plugin unload admitted");
  assert(fixture.platform.silent_degradation === false, "platform degraded silently");
  assert(!fixture.composition.surface_dependency, "Surface dependency admitted");
  assert(!fixture.composition.transfer_dependency, "transfer dependency admitted");
  assert(!fixture.composition.bridge_dependency, "bridge dependency admitted");
  assert(!fixture.composition.package_manager_publication, "publication claimed");
  assert(fixture.validation.live_library_mutated === false, "live library mutation claimed");
}

function verifyLonghornBoundary(): void {
  const library = longhornRead("crates/longhorn-native-content-isolated-window/src/lib.rs");
  for (const token of [
    "IsolatedWindowAdapter",
    "IsolatedWindowRuntime",
    "HelperSnapshot",
    "TeardownOutcome",
  ]) assertContains(library, token, "Longhorn isolated-window export");
  const runtime = longhornRead("crates/longhorn-native-content-isolated-window/src/runtime.rs");
  for (const token of [
    "callback installed before launch",
    "OwnerProcessTerminated",
    "HelperLost",
    "ContentRequest",
  ]) assertContains(runtime, token, "Longhorn isolated runtime");
  const adapter = longhornRead("crates/longhorn-native-content-isolated-window/src/adapter.rs");
  for (const token of [
    "ListenerInstalled",
    "compare_generation",
    "MAX_PENDING_CONTENT_REQUESTS",
    "ResizeCycleSuppressed",
    "teardown_reports",
  ]) assertContains(adapter, token, "Longhorn isolated adapter");
  const protocol = longhornRead("crates/longhorn-native-content-isolated-window/src/protocol.rs");
  for (const forbidden of ["product_id", "variant_id", "install_id", "plugin", "raw_handle"]) {
    assertExcludes(protocol, forbidden, "Longhorn helper protocol");
  }
}

function verifySoundcheckCoordination(): void {
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
  for (const forbidden of ["longhorn-surfaces", "longhorn-transfer", "longhorn-bridge"]) {
    assertExcludes(desktop, forbidden, "Soundcheck native-content dependency graph");
  }
  const host = soundcheckRead("src-tauri/src/plugin_inspection_coordination.rs");
  for (const token of [
    'ISLAND_PREFIX: &str = "island:soundcheck-plugin-inspection:"',
    'KIND_ID: &str = "soundcheck:plugin-inspection"',
    "ISOLATED_WINDOW_CAPABILITIES",
    "IsolatedWindowAdapter::new",
    "NativeContentProtocolHost::new",
    '"bootstrap_apply"',
    "ContentSizeDecision::Constrained",
    "admit_fresh_observation",
    "FocusIntent::Unchanged",
  ]) assertContains(host, token, "Soundcheck coordination host");
  const productionHost = host.split("#[cfg(test)]", 1)[0] ?? host;
  for (const forbidden of ["product_id", "variant_id", "install_id", "parent_view", "raw_handle"]) {
    assertExcludes(productionHost, forbidden, "Soundcheck Longhorn host boundary");
  }
}

function verifyHelperLifecycle(): void {
  const process = soundcheckRead("src-tauri/src/plugin_inspection_process.rs");
  for (const token of [
    "HELPER_REQUEST_HISTORY_LIMIT: usize = 64",
    "watch_parent",
    "STARTUP_SLOW_AFTER",
    "STARTUP_HARD_TIMEOUT",
    "emit_helper_lost",
    '"soundcheck:request-gap"',
    '"soundcheck:helper-crashed"',
    "TeardownOutcome::OwnerProcessTerminated",
    "state.island_id != handle.island_id || state.generation != handle.generation",
    "helper_requests_are_admitted_only_for_the_exact_generation",
    "helper_request_history_is_bounded_without_reordering",
  ]) assertContains(process, token, "Soundcheck helper lifecycle");
  const plugin = soundcheckRead("src-tauri/src/plugin_inspection.rs");
  for (const token of [
    "accept_plugin_resize",
    "processor.set_size",
    "report_helper_resize",
    "report_helper_close",
    "exit_inspection_helper_immediately",
    "install_header",
    "capture_plugin_screenshot",
  ]) assertContains(plugin, token, "retained inspection behavior");
  assertContains(
    plugin,
    "plugin UI inspection is currently available on macOS only",
    "unsupported-platform statement",
  );
}

function verifyRenderer(): void {
  const manifest = JSON.parse(soundcheckRead("package.json")) as {
    dependencies: Record<string, string>;
  };
  for (const dependency of fixture.composition.typescript) {
    assert(
      manifest.dependencies[dependency]?.startsWith("file:../longhorn/"),
      `Soundcheck TypeScript dependency missing: ${dependency}`,
    );
  }
  const observer = soundcheckRead("src/lib/inspection-native-content.ts");
  for (const token of [
    "createTauriNativeContentPort",
    "new NativeContentClient(port, islandId).connect(listener)",
    "observationOnly",
    "current: () => connection.current()",
    "dispose: () => connection.dispose()",
  ]) assertContains(observer, token, "Soundcheck renderer observer");
  assertExcludes(observer, "updateDesired:", "renderer desired mutation");
  assertExcludes(observer, "decideContentSize:", "renderer size mutation");
  const test = soundcheckRead("src/lib/inspection-native-content.test.ts");
  assertContains(test, 'calls).toEqual(["listen", "connect"]', "listener-first test");
  assertContains(test, '"updateDesired" in observation', "mutation exclusion test");
  const api = soundcheckRead("src/lib/api.ts");
  for (const token of [
    "openPluginInspectionIsland",
    "openHostedProductInspectionIsland",
    "openBrowsePluginInspectionIsland",
    "openBrowseHostedProductInspectionIsland",
  ]) {
    assertContains(api, token, "renderer island identity API");
  }
}

function verifyRetainedAuthorities(): void {
  const plugin = soundcheckRead("src-tauri/src/plugin_inspection.rs");
  for (const token of [
    "InProcessClapProcessor",
    "InProcessVst3Processor",
    "InProcessVst3Editor",
    "InProcessAuProcessor",
    "InspectionPlayback::start",
    "midi_queue",
  ]) assertContains(plugin, token, "Signal authority remains downstream");
  const process = soundcheckRead("src-tauri/src/plugin_inspection_process.rs");
  for (const token of [
    "begin_browse_usage",
    "finalize_browse_usage",
    "emit_screenshot_changed",
    '"KEEPSAKE_MAC_DISPOSABLE_EDITOR"',
  ]) assertContains(process, token, "Soundcheck process authority remains downstream");
  const signal = signalRead("crates/signal-plugin-bridge/src/in_process.rs");
  for (const token of ["InProcessClapProcessor", "InProcessVst3Processor"]) {
    assertContains(signal, token, "retained Signal bridge source");
  }
}

function soundcheckRead(path: string): string {
  return git(soundcheckRoot, ["show", `${soundcheckCommit}:${path}`]);
}

function soundcheckReadRaw(path: string): Buffer {
  return execFileSync("git", ["show", `${soundcheckCommit}:${path}`], { cwd: soundcheckRoot });
}

function signalRead(path: string): string {
  return git(signalRoot, ["show", `${fixture.sources.signal_commit}:${path}`]);
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
