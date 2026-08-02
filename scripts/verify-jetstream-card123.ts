import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type Fixture = {
  schema: string;
  outcome: string;
  sources: Record<string, string>;
  composition: {
    canonical_application_id: string;
    typescript_packages: string[];
    rust_packages: string[];
    forbidden_systems: string[];
    package_manager_publication: boolean;
  };
  coordination: Record<string, boolean | number | string>;
  lifecycle: Record<string, boolean | string>;
  platforms: Record<string, string>;
  authority: Record<string, string[]>;
  validation: Record<string, boolean | number | string | string[]>;
};

const longhornRoot = resolve(import.meta.dir, "..");
const jetstreamRoot = resolve(
  process.env.JETSTREAM_REPO ?? resolve(longhornRoot, "../jetstream"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/jetstream-card123/backing-surface-coordination-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifySourcesAndScope();
verifyComposition();
verifyCoordinationHost();
verifyRendererLifecycle();
verifyNativeOwnershipAndTeardown();
verifyInputBoundary();
verifyRetainedAuthority();
verifyFocusedTests();

console.log(JSON.stringify({
  schema: fixture.schema,
  outcome: fixture.outcome,
  source: fixture.sources.jetstream_cutover_commit,
  composition: {
    rust: fixture.composition.rust_packages.length,
    typescript: fixture.composition.typescript_packages.length,
  },
  coordination: fixture.coordination,
  lifecycle: fixture.lifecycle,
  platforms: fixture.platforms,
  authority: fixture.authority,
  validation: fixture.validation,
  packageManagerPublication: false,
}, null, 2));

function verifyFixture(): void {
  equal(
    fixture.schema,
    "longhorn.jetstream-backing-surface-coordination-cutover.v1",
    "fixture schema",
  );
  equal(
    fixture.outcome,
    "pass-with-unrelated-workspace-clippy-failures",
    "fixture outcome",
  );
  equal(
    fixture.composition.canonical_application_id,
    "com.inflatablecookie.jetstream.editor",
    "canonical application id",
  );
  assert(!fixture.composition.package_manager_publication, "publication claimed");
  equal(fixture.coordination.mechanism, "backing_surface", "mechanism");
  equal(fixture.coordination.detach_policy, "reversible", "detach policy");
  assert(fixture.coordination.full_host_storage === true, "full-host storage missing");
  assert(fixture.coordination.separate_physical_clip === true, "clip collapsed into storage");
  assert(fixture.lifecycle.host_destroy_invalidates === true, "host invalidation missing");
  assert(fixture.lifecycle.raw_set_viewport_endpoint === false, "raw viewport endpoint retained");
  assert(fixture.lifecycle.process_lifetime_native_leak === false, "native leak retained");
  assert(
    fixture.validation.aggregate_failure_paths_changed_by_card === false,
    "unrelated aggregate failures were changed",
  );
}

function verifySourcesAndScope(): void {
  equal(
    git(jetstreamRoot, ["rev-parse", "HEAD"]),
    fixture.sources.jetstream_cutover_commit,
    "Jetstream HEAD",
  );
  for (const [root, commit] of [
    [longhornRoot, fixture.sources.longhorn_selected_source_commit],
    [jetstreamRoot, fixture.sources.jetstream_prior_commit],
    [jetstreamRoot, fixture.sources.jetstream_cutover_commit],
  ] as const) {
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  git(jetstreamRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.jetstream_prior_commit,
    fixture.sources.jetstream_cutover_commit,
  ]);
  equal(
    sha256(jetstreamReadRaw("Cargo.lock")),
    fixture.sources.jetstream_cargo_lock_sha256,
    "Jetstream Cargo lock",
  );
  equal(
    sha256(jetstreamReadRaw("editor-ui/package-lock.json")),
    fixture.sources.jetstream_package_lock_sha256,
    "Jetstream package lock",
  );
  equal(git(jetstreamRoot, ["status", "--porcelain"]), "", "Jetstream worktree");

  const expected = [
    "Cargo.lock",
    "crates/jetstream-editor-tauri/Cargo.toml",
    "crates/jetstream-editor-tauri/README.md",
    "crates/jetstream-editor-tauri/src/commands.rs",
    "crates/jetstream-editor-tauri/src/lib.rs",
    "crates/jetstream-editor-tauri/src/native_content.rs",
    "crates/jetstream-editor-tauri/src/state.rs",
    "crates/jetstream-editor-tauri/src/surface.rs",
    "crates/jetstream-editor-tauri/tests/ipc.rs",
    "editor-ui/README.md",
    "editor-ui/package-lock.json",
    "editor-ui/package.json",
    "editor-ui/src/Editor.svelte",
    "editor-ui/src/bridge.ts",
    "editor-ui/src/native-content.ts",
    "editor-ui/tests/editor-panels.test.ts",
    "editor-ui/tests/native-content.test.ts",
    "editor-ui/tests/viewport-input.test.ts",
  ];
  const changed = git(jetstreamRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
  ]).split("\n").filter(Boolean).sort();
  equal(changed, expected.sort(), "Card 123 path inventory");

  for (const path of fixture.validation.aggregate_failure_paths as string[]) {
    equal(
      git(jetstreamRoot, [
        "diff",
        "--name-only",
        `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
        "--",
        path,
      ]),
      "",
      `aggregate Clippy failure path ${path}`,
    );
  }
}

function verifyComposition(): void {
  const tauri = JSON.parse(
    jetstreamRead("crates/jetstream-editor-tauri/tauri.conf.json"),
  ) as { identifier: string };
  equal(
    tauri.identifier,
    fixture.composition.canonical_application_id,
    "Tauri application id",
  );

  const manifest = JSON.parse(jetstreamRead("editor-ui/package.json")) as {
    dependencies: Record<string, string>;
  };
  const selectedTs = Object.keys(manifest.dependencies)
    .filter((name) => name.startsWith("@longhorn/"))
    .sort();
  equal(selectedTs, [...fixture.composition.typescript_packages].sort(), "TypeScript graph");

  const cargo = jetstreamRead("crates/jetstream-editor-tauri/Cargo.toml");
  const selectedRust = [...cargo.matchAll(/^(longhorn-[\w-]+)\s*=\s*\{/gm)]
    .map((match) => match[1])
    .sort();
  equal(selectedRust, [...fixture.composition.rust_packages].sort(), "Rust graph");

  const tree = command("cargo", [
    "tree", "-p", "jetstream-editor-tauri", "--edges", "normal", "--prefix", "none",
  ], jetstreamRoot);
  for (const system of fixture.composition.forbidden_systems) {
    assert(!manifest.dependencies[`@longhorn/${system}`], `renderer resolves ${system}`);
    assert(!tree.includes(`longhorn-${system} `), `Rust graph resolves ${system}`);
  }
}

function verifyCoordinationHost(): void {
  const host = jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs");
  for (const token of [
    'EDITOR_ISLAND_ID: &str = "island:jetstream.editor.viewport"',
    'EDITOR_HOST_WINDOW_ID: &str = "window:editor"',
    'NativeContentKindId::new("jetstream:wgpu-editor")',
    "BACKING_SURFACE_CAPABILITIES",
    "BackingSurfaceAdapter<SurfaceRuntime>",
    "NativeContentProtocolHost",
    "DesiredVisibility::Hidden",
    'VisibilityReasonId::new("page_not_loaded")',
    "InputRoutingMode::Disabled",
    "reconcile_authority",
    ".complete_apply(",
    ".admit_observation(",
    ".refresh_host_geometry(",
    ".update_host_focus(",
    ".host_destroyed(",
  ]) contains(host, token, "checked native-content host");
  for (const endpoint of [
    "longhorn_native_content_connect",
    "longhorn_native_content_snapshot",
    "longhorn_native_content_update_desired",
    "longhorn_native_content_decide_size",
  ]) contains(host, endpoint, "native-content endpoint");
  excludes(host, "wgpu::", "Longhorn coordination boundary");
  excludes(host, "use objc2", "Longhorn coordination boundary");
  excludes(host, "*mut", "Longhorn coordination boundary");

  const app = jetstreamRead("crates/jetstream-editor-tauri/src/lib.rs");
  contains(app, "EditorNativeContentHost::production", "production host assembly");
  contains(app, "native_content.take_embedded()", "consumer-owned WGPU take");
  contains(app, "presentation_enabled", "presentation gate");
  contains(app, "native_content.frame_presented()", "frame evidence");
  contains(app, "WindowEvent::ScaleFactorChanged", "live scale boundary");
  contains(app, "WindowEvent::Destroyed", "window destroy boundary");
  contains(app, "JETSTREAM_EDITOR_NO_SURFACE", "no-surface policy");
  excludes(app, "commands::set_viewport", "raw viewport endpoint");
}

function verifyRendererLifecycle(): void {
  const session = jetstreamRead("editor-ui/src/native-content.ts");
  for (const token of [
    "new NativeContentClient(",
    "createTauriNativeContentPort({",
    "new NativeContentSession({",
    "EDITOR_NATIVE_CONTENT_ISLAND",
    "visibility: { state: 'visible' }",
    "inputRouting: 'renderer_forwarded'",
    "target.scaleFactor()",
    "target.onScaleChanged",
    "scaleFromRatio",
  ]) contains(session, token, "renderer native-content session");

  const editor = jetstreamRead("editor-ui/src/Editor.svelte");
  for (const token of [
    "createEditorNativeContentSession()",
    "bindNativeScale(nativeContent)",
    "await nativeContent.start()",
    "void nativeContent.stop()",
    "use:nativeContentViewport={nativeContent}",
  ]) contains(editor, token, "Svelte native-content lifecycle");
  excludes(editor, "reportViewport", "superseded viewport listener");

  const tests = jetstreamRead("editor-ui/tests/native-content.test.ts");
  for (const token of [
    "['listen', 'connect', 'update']",
    "remounts with a fresh epoch",
    "expect(transport.epoch).toBe(2)",
    "uses fresh Tauri scale evidence",
    "expect(transport.listeners.size).toBe(0)",
  ]) contains(tests, token, "renderer lifecycle proof");
}

function verifyNativeOwnershipAndTeardown(): void {
  const surface = jetstreamRead("crates/jetstream-editor-tauri/src/surface.rs");
  for (const token of [
    "pub struct NativeAttachment",
    "pub struct AttachedSurface",
    "view.removeFromSuperview()",
    "Retained::into_raw(view)",
    "Retained::<NSView>::from_raw",
    "run_on_main_thread",
    "surface embedding not implemented",
  ]) contains(surface, token, "consumer native ownership");
  excludes(surface, "std::mem::forget(view)", "process-lifetime native leak");

  const host = jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs");
  for (const token of [
    "if let Some(thread) = thread",
    "if let Some(native) = native",
    "state.pending_embedded = None",
    "state.snapshot.native_storage_attached = false",
    "consumer_runtime_keeps_full_host_storage_separate_from_the_checked_clip",
    "absent_diagnostic_mode_still_invalidates_on_host_destroy",
    "frame_evidence_is_generation_scoped_and_product_free",
  ]) contains(host, token, "reversible consumer teardown");
  const join = host.indexOf("if let Some(thread) = thread");
  const detach = host.indexOf("if let Some(native) = native");
  assert(join >= 0 && detach > join, "native detach precedes render-thread join");
}

function verifyInputBoundary(): void {
  const commands = jetstreamRead("crates/jetstream-editor-tauri/src/commands.rs");
  for (const token of [
    "fn admits_input",
    ".admits_css_point(x, y, scale)",
    "host_x: f64",
    "host_y: f64",
    "if !stopped && !admits_input",
    "if buttons != 0 && !admits_input",
  ]) contains(commands, token, "semantic input admission");

  const bridge = jetstreamRead("editor-ui/src/bridge.ts");
  for (const token of [
    "hostX: e.clientX",
    "hostY: e.clientY",
    "viewportGizmoDrag(x, y, false, true, e.clientX, e.clientY)",
  ]) contains(bridge, token, "host-space input evidence");
  excludes(bridge, "set_viewport", "raw viewport command");

  const ipc = jetstreamRead("crates/jetstream-editor-tauri/tests/ipc.rs");
  for (const token of [
    "native_content_protocol_stays_checked_in_no_surface_mode",
    '"stale_client_epoch"',
    "disabled input reached the engine",
    "disabled click reached the engine",
    "disabled gizmo input reached the engine",
  ]) contains(ipc, token, "native IPC and input gate");
}

function verifyRetainedAuthority(): void {
  const host = jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs");
  for (const token of [
    "crate::surface::attach",
    "pending_embedded: Option<Embedded>",
    "native: Option<NativeAttachment>",
    "render_thread: Option<JoinHandle<()>>",
  ]) contains(host, token, "Jetstream consumer port");

  const changedOutsideComposition = git(jetstreamRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
    "--",
    "crates/jetstream-editor",
    "crates/jetstream-renderer",
    "crates/jetstream-world",
    "crates/jetstream-gizmo",
  ]);
  equal(changedOutsideComposition, "", "engine/renderer/world/gizmo authority scope");
}

function verifyFocusedTests(): void {
  command("cargo", [
    "test", "-p", "jetstream-editor-tauri", "--lib", "--test", "ipc",
  ], jetstreamRoot);
  command("effigy", ["editor:test"], jetstreamRoot);
}

function jetstreamRead(path: string): string {
  return readFileSync(resolve(jetstreamRoot, path), "utf8");
}

function jetstreamReadRaw(path: string): Buffer {
  return readFileSync(resolve(jetstreamRoot, path));
}

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

function git(root: string, args: string[]): string {
  return command("git", args, root).trim();
}

function command(executable: string, args: string[], cwd: string): string {
  return execFileSync(executable, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function contains(value: string, token: string, label: string): void {
  assert(value.includes(token), `${label} missing ${JSON.stringify(token)}`);
}

function excludes(value: string, token: string, label: string): void {
  assert(!value.includes(token), `${label} contains ${JSON.stringify(token)}`);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  const left = JSON.stringify(actual);
  const right = JSON.stringify(expected);
  assert(left === right, `${label}: expected ${right}, received ${left}`);
}

function assert(condition: boolean, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
