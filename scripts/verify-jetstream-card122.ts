import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
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
  bridge: Record<string, boolean | number | string | string[]>;
  commands: Record<string, boolean | number>;
  typed_ipc: string[];
  authority: Record<string, string[]>;
  validation: Record<string, boolean | number | string>;
};

const longhornRoot = resolve(import.meta.dir, "..");
const jetstreamRoot = resolve(
  process.env.JETSTREAM_REPO ?? resolve(longhornRoot, "../jetstream"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/jetstream-card122/bridge-command-keyboard-cutover-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifySourcesAndScope();
verifyComposition();
verifyBridge();
verifyCommandsAndKeyboard();
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
  bridge: fixture.bridge,
  commands: fixture.commands,
  authority: fixture.authority,
  validation: fixture.validation,
  packageManagerPublication: false,
}, null, 2));

function verifyFixture(): void {
  equal(
    fixture.schema,
    "longhorn.jetstream-bridge-command-keyboard-cutover.v1",
    "fixture schema",
  );
  equal(
    fixture.outcome,
    "pass-with-unrelated-workspace-clippy-failure",
    "fixture outcome",
  );
  equal(
    fixture.composition.canonical_application_id,
    "com.inflatablecookie.jetstream.editor",
    "canonical application id",
  );
  assert(!fixture.composition.package_manager_publication, "publication claimed");
  assert(fixture.bridge.listener_before_snapshot === true, "listener-first bridge missing");
  assert(fixture.bridge.gap_resync === true, "gap recovery missing");
  assert(fixture.bridge.stale_session_rejected === true, "stale session remains live");
  assert(fixture.commands.registry_commands === 16, "command inventory drifted");
  assert(fixture.commands.default_bindings === 8, "keymap inventory drifted");
  assert(
    fixture.commands.generic_execute_by_string_endpoint === false,
    "generic execute-by-string endpoint claimed",
  );
  assert(
    fixture.validation.aggregate_failure_path_changed_by_card === false,
    "unrelated aggregate failure was changed",
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
    "crates/jetstream-editor-tauri/src/bridge_host.rs",
    "crates/jetstream-editor-tauri/src/command_system.rs",
    "crates/jetstream-editor-tauri/src/commands.rs",
    "crates/jetstream-editor-tauri/src/host.rs",
    "crates/jetstream-editor-tauri/src/lib.rs",
    "crates/jetstream-editor-tauri/src/publish.rs",
    "crates/jetstream-editor-tauri/src/state.rs",
    "crates/jetstream-editor-tauri/tests/ipc.rs",
    "editor-ui/README.md",
    "editor-ui/index.html",
    "editor-ui/package-lock.json",
    "editor-ui/package.json",
    "editor-ui/src/Editor.svelte",
    "editor-ui/src/bridge.ts",
    "editor-ui/src/commands.ts",
    "editor-ui/src/shortcuts.ts",
    "editor-ui/tests/bridge-stream.test.ts",
    "editor-ui/tests/editor-panels.test.ts",
    "editor-ui/tests/shortcuts.test.ts",
  ];
  const changed = git(jetstreamRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
  ]).split("\n").filter(Boolean).sort();
  equal(changed, expected.sort(), "Card 122 path inventory");

  const aggregateFailurePath = String(fixture.validation.aggregate_failure_path);
  equal(
    git(jetstreamRoot, [
      "diff",
      "--name-only",
      `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
      "--",
      aggregateFailurePath,
    ]),
    "",
    "aggregate Clippy failure path",
  );
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

function verifyBridge(): void {
  const host = jetstreamRead("crates/jetstream-editor-tauri/src/bridge_host.rs");
  for (const token of [
    'EDITOR_STATE_DOMAIN: &str = "jetstream.editor"',
    'READ_CAPABILITY: &str = "editor_state.read"',
    'SUBSCRIBE_CAPABILITY: &str = "editor_state.subscribe"',
    ".checked_next()",
    "BridgeEventEnvelope::new",
    "publish_domain_event(&event)",
    "ReadAuthority::Authoritative",
    "WriteAuthority::None",
    "ExecutionAuthority::None",
    "AuthorityEpoch::new(1)",
  ]) contains(host, token, "checked bridge host");

  const app = jetstreamRead("crates/jetstream-editor-tauri/src/lib.rs");
  for (const endpoint of [
    "longhorn_tauri_bridge::longhorn_bridge_hello",
    "longhorn_tauri_bridge::longhorn_bridge_authority",
    "longhorn_tauri_bridge::longhorn_bridge_resync",
  ]) contains(app, endpoint, "bridge endpoint");
  for (const endpoint of [
    "longhorn_bridge_command",
    "longhorn_bridge_query",
    "longhorn_bridge_cancel",
  ]) excludes(app, endpoint, "generic bridge execution endpoint");

  const renderer = jetstreamRead("editor-ui/src/bridge.ts");
  for (const token of [
    "connectTauriBridge(",
    "connectBridgeStream({",
    "new TauriBridgeStreamSource(",
    "apply: (_current, event) => event",
    "onSnapshot: ({ payload }) => handler(payload)",
    "void stream?.dispose()",
    "bridgeCodec(parseEngineState)",
  ]) contains(renderer, token, "checked renderer bridge");
  excludes(renderer, "jetstream:state", "superseded raw state event");

  const rendererTests = jetstreamRead("editor-ui/tests/bridge-stream.test.ts");
  for (const token of [
    "['hello', 'listen', 'snapshot']",
    "a sequence gap reloads the whole snapshot",
    "Must not apply directly",
    "remount negotiates a new session",
    "expect(transport.listeners.size).toBe(0)",
  ]) contains(rendererTests, token, "renderer bridge lifecycle");
  const ipc = jetstreamRead("crates/jetstream-editor-tauri/tests/ipc.rs");
  for (const token of [
    "remount reused a bridge session",
    "superseded bridge session still had authority",
    '"longhorn_bridge_resync"',
  ]) contains(ipc, token, "native bridge lifecycle");
}

function verifyCommandsAndKeyboard(): void {
  const system = jetstreamRead("crates/jetstream-editor-tauri/src/command_system.rs");
  for (const token of [
    "builder.seal()",
    "CommandAdmissionEngine::new(&self.registry).execute(",
    "CommandArgumentKind::String",
    "QueuedEditorCommand::ImportAsset(path.clone())",
    "assert_eq!(registry.commands().count(), 16)",
    "assert_eq!(keymap.bindings().count(), 8)",
    "execution_reloads_availability_before_queueing_product_work",
  ]) contains(system, token, "sealed command system");

  const client = jetstreamRead("editor-ui/src/commands.ts");
  for (const token of [
    "new CommandController({",
    "routeCommandKeyboardEvent(event",
    "platform: this.#platform",
    "this.#target.addEventListener('keydown'",
    "this.#target.removeEventListener('keydown'",
    "assetImportArguments(path: string)",
  ]) contains(client, token, "command client");
  assert(
    !existsSync(resolve(jetstreamRoot, "editor-ui/src/shortcuts.ts")),
    "superseded shortcut table still exists",
  );

  const tests = jetstreamRead("editor-ui/tests/shortcuts.test.ts");
  for (const token of [
    "routes physical %s",
    "uses physical code rather than the layout-dependent key value",
    "repeat: true",
    "isComposing: true",
    "event.defaultPrevented",
    "teardown removes the listener without touching the engine",
    "toBe('⌘S')",
  ]) contains(tests, token, "keyboard regression matrix");
}

function verifyRetainedAuthority(): void {
  const app = jetstreamRead("crates/jetstream-editor-tauri/src/lib.rs");
  for (const endpoint of fixture.typed_ipc) contains(app, endpoint, "typed IPC");
  excludes(app, "commands::editor_command,", "raw editor command endpoint");

  const state = jetstreamRead("crates/jetstream-editor-tauri/src/state.rs");
  contains(state, "pub commands: Vec<QueuedEditorCommand>", "typed render-thread queue");
  excludes(state, "pub commands: Vec<String>", "string command queue");
  const host = jetstreamRead("crates/jetstream-editor-tauri/src/host.rs");
  contains(host, "command.dispatch_id()", "consumer-owned product dispatch");

  const changedOutsideComposition = git(jetstreamRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.jetstream_prior_commit}..${fixture.sources.jetstream_cutover_commit}`,
    "--",
    "crates/jetstream-editor",
    "crates/jetstream-renderer",
    "crates/jetstream-world",
  ]);
  equal(changedOutsideComposition, "", "engine/renderer/world authority scope");
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

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}
