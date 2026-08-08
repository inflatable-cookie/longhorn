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
    hierarchy: string[];
    typescript_packages: string[];
    rust_packages: string[];
    forbidden_systems: string[];
    package_manager_publication: boolean;
  };
  artifacts: Record<string, boolean | number | string>;
  peers: Record<string, string | boolean>;
  capabilities: Record<string, string[] | string | number | boolean>;
  duplicates: Record<string, boolean | number>;
  rollback: Record<string, boolean | number | string>;
  authority: Record<string, string[]>;
  platforms: Record<string, string>;
  validation: Record<string, boolean | number | string | string[]>;
};

const longhornRoot = resolve(import.meta.dir, "..");
const jetstreamRoot = resolve(process.env.JETSTREAM_REPO ?? resolve(longhornRoot, "../jetstream"));
const poodleRoot = resolve(process.env.POODLE_REPO ?? resolve(longhornRoot, "../poodle"));
const signalRoot = resolve(process.env.SIGNAL_REPO ?? resolve(longhornRoot, "../signal"));
const fixture = JSON.parse(readFileSync(resolve(
  longhornRoot,
  "fixtures/migration/jetstream-card124/conformance-closeout-v1.json",
), "utf8")) as Fixture;

verifyFixture();
verifySourcesAndArtifacts();
verifyCompositionAndPeers();
verifyCapabilitiesAndDuplicates();
verifyRetainedAuthorityAndRollback();
verifyFocusedSuites();

console.log(JSON.stringify({
  schema: fixture.schema,
  outcome: fixture.outcome,
  source: fixture.sources.jetstream_closeout_commit,
  composition: {
    hierarchy: fixture.composition.hierarchy,
    typescript: fixture.composition.typescript_packages.length,
    rust: fixture.composition.rust_packages.length,
    forbiddenSystemsAbsent: fixture.composition.forbidden_systems,
  },
  artifacts: fixture.artifacts,
  peers: fixture.peers,
  capabilities: fixture.capabilities,
  duplicates: fixture.duplicates,
  rollback: fixture.rollback,
  authority: fixture.authority,
  platforms: fixture.platforms,
  packageManagerPublication: false,
}, null, 2));

function verifyFixture(): void {
  equal(fixture.schema, "longhorn.jetstream-migration-conformance-closeout.v1", "fixture schema");
  equal(fixture.outcome, "pass-with-unrelated-workspace-clippy-failures", "fixture outcome");
  equal(fixture.composition.canonical_application_id, "com.inflatablecookie.jetstream.editor", "app id");
  equal(fixture.composition.hierarchy, ["window", "native-content-island", "backing-surface"], "hierarchy");
  assert(!fixture.composition.package_manager_publication, "package publication claimed");
  assert(fixture.artifacts.jetstream_selected_sources_unchanged === true, "selected source drift claimed");
  assert(fixture.artifacts.one_svelte_runtime === true, "single Svelte runtime not proved");
  assert(fixture.artifacts.original_workspace_resolution === false, "artifact resolved original workspace");
  assert(fixture.artifacts.sibling_source_resolution === false, "artifact resolved sibling source");
  assert(fixture.rollback.dual_write === false, "rollback used dual writes");
  assert(fixture.rollback.silent_fallback === false, "rollback used silent fallback");
  assert(fixture.rollback.temporary_worktrees_removed === true, "rollback worktree retained");
  assert(fixture.rollback.live_repositories_mutated === false, "rollback mutated live repositories");
  assert(fixture.validation.aggregate_failure_paths_changed === false, "unrelated lint paths changed");
}

function verifySourcesAndArtifacts(): void {
  equal(git(jetstreamRoot, ["rev-parse", "HEAD"]), fixture.sources.jetstream_closeout_commit, "Jetstream HEAD");
  equal(git(jetstreamRoot, ["status", "--porcelain"]), "", "Jetstream worktree");
  for (const [root, commit] of [
    [longhornRoot, fixture.sources.longhorn_artifact_source_commit],
    [longhornRoot, fixture.sources.longhorn_evidence_parent_commit],
    [poodleRoot, fixture.sources.poodle_artifact_source_commit],
    [jetstreamRoot, fixture.sources.jetstream_previous_authority_commit],
    [jetstreamRoot, fixture.sources.jetstream_bridge_cutover_commit],
    [jetstreamRoot, fixture.sources.jetstream_closeout_commit],
    [signalRoot, fixture.sources.signal_rollback_source_commit],
  ] as const) git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  git(jetstreamRoot, ["merge-base", "--is-ancestor", fixture.sources.jetstream_previous_authority_commit, fixture.sources.jetstream_closeout_commit]);
  equal(sha256(jetstreamReadRaw("Cargo.lock")), fixture.sources.jetstream_cargo_lock_sha256, "Cargo lock");
  equal(sha256(jetstreamReadRaw("editor-ui/package-lock.json")), fixture.sources.jetstream_package_lock_sha256, "renderer lock");

  const selectedLonghorn = [
    "packages/core", "packages/bridge", "packages/commands", "packages/native-content",
    "packages/native-content-svelte", "packages/tauri", "crates/longhorn-core",
    "crates/longhorn-bridge", "crates/longhorn-command", "crates/longhorn-native-content",
    "crates/longhorn-native-content-backing-surface", "crates/longhorn-tauri-bridge",
  ];
  equal(git(longhornRoot, ["diff", "--name-only", `${fixture.sources.longhorn_artifact_source_commit}..HEAD`, "--", ...selectedLonghorn]), "", "selected Longhorn source drift");
  equal(git(longhornRoot, ["status", "--porcelain", "--", ...selectedLonghorn]), "", "selected Longhorn worktree");
  const selectedPoodle = [
    "packages/core", "packages/styles", "packages/svelte/tokens",
    "packages/svelte/icons-lucide", "packages/svelte/components",
  ];
  equal(git(poodleRoot, ["diff", "--name-only", `${fixture.sources.poodle_artifact_source_commit}..HEAD`, "--", ...selectedPoodle]), "", "selected Poodle source drift");
  equal(git(poodleRoot, ["status", "--porcelain", "--", ...selectedPoodle]), "", "selected Poodle worktree");

  const admission = JSON.parse(readFileSync(resolve(longhornRoot, String(fixture.artifacts.admission_fixture)), "utf8")) as {
    artifact_sets: Record<string, string>;
    artifacts: Record<string, number>;
    graphs: { jetstream: { typescript: number; rust: number } };
    audits: Record<string, boolean>;
  };
  equal(admission.artifact_sets.poodle, fixture.artifacts.poodle_set, "Poodle artifact set");
  equal(admission.artifact_sets.longhorn_typescript, fixture.artifacts.longhorn_typescript_set, "TypeScript artifact set");
  equal(admission.artifact_sets.longhorn_rust, fixture.artifacts.longhorn_rust_set, "Rust artifact set");
  equal(admission.artifact_sets.rust_lock, fixture.artifacts.rust_lock, "artifact Rust lock");
  equal(admission.artifacts.poodle, fixture.artifacts.poodle_packages, "Poodle artifact count");
  equal(admission.artifacts.longhorn_typescript, fixture.artifacts.typescript_packages, "TypeScript artifact count");
  equal(admission.artifacts.longhorn_rust, fixture.artifacts.rust_packages, "Rust artifact count");
  equal(admission.graphs.jetstream, { typescript: 6, rust: 6 }, "admitted Jetstream graph");
  assert(admission.audits.one_svelte_runtime_per_renderer, "admission has duplicate Svelte runtime");
  assert(!admission.audits.original_workspace_resolution, "admission used original workspace");
  assert(!admission.audits.sibling_source_resolution, "admission used sibling source");
  assert(!admission.audits.package_manager_publication, "admission published packages");
}

function verifyCompositionAndPeers(): void {
  const tauri = JSON.parse(jetstreamRead("crates/jetstream-editor-tauri/tauri.conf.json")) as { identifier: string };
  equal(tauri.identifier, fixture.composition.canonical_application_id, "Tauri app id");
  const renderer = JSON.parse(jetstreamRead("editor-ui/package.json")) as { dependencies: Record<string, string> };
  const selectedTs = Object.keys(renderer.dependencies).filter((name) => name.startsWith("@inflatable-cookie/longhorn-")).sort();
  equal(selectedTs, [...fixture.composition.typescript_packages].sort(), "TypeScript graph");
  const cargo = jetstreamRead("crates/jetstream-editor-tauri/Cargo.toml");
  const selectedRust = [...cargo.matchAll(/^(longhorn-[\w-]+)\s*=\s*\{/gm)].map((match) => match[1]).sort();
  equal(selectedRust, [...fixture.composition.rust_packages].sort(), "Rust graph");
  const tree = command("cargo", ["tree", "-p", "jetstream-editor-tauri", "--edges", "normal", "--prefix", "none"], jetstreamRoot);
  for (const system of fixture.composition.forbidden_systems) {
    assert(!renderer.dependencies[`@inflatable-cookie/longhorn-${system}`], `renderer resolves forbidden ${system}`);
    assert(!tree.includes(`longhorn-${system} `), `Rust graph resolves forbidden ${system}`);
  }

  const commands = packageManifest("commands");
  const nativeSvelte = packageManifest("native-content-svelte");
  const tauriPackage = packageManifest("tauri");
  equal(commands.peerDependencies?.svelte, fixture.peers.svelte, "commands Svelte peer");
  equal(commands.peerDependencies?.["@poodle/svelte"], fixture.peers.poodle_svelte, "commands Poodle peer");
  assert(commands.peerDependenciesMeta?.["@poodle/svelte"]?.optional === fixture.peers.poodle_svelte_optional, "commands Poodle optionality drift");
  equal(nativeSvelte.peerDependencies?.svelte, fixture.peers.svelte, "native-content Svelte peer");
  equal(tauriPackage.peerDependencies?.["@tauri-apps/api"], fixture.peers.tauri_api, "Tauri API peer");
}

function verifyCapabilitiesAndDuplicates(): void {
  const bridge = jetstreamRead("crates/jetstream-editor-tauri/src/bridge_host.rs");
  for (const capability of fixture.capabilities.bridge as string[]) contains(bridge, `\"${capability}\"`, "bridge capability");
  const commandSystem = jetstreamRead("crates/jetstream-editor-tauri/src/command_system.rs");
  equal([...commandSystem.matchAll(/required_capabilities:\s*Vec::new\(\)/g)].length > 0, true, "command capability declaration");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs"), "BACKING_SURFACE_CAPABILITIES", "backing-surface capability");

  const rendererSources = ["editor-ui/src/Editor.svelte", "editor-ui/src/bridge.ts", "editor-ui/src/commands.ts", "editor-ui/src/native-content.ts"].map(jetstreamRead).join("\n");
  const nativeSources = ["crates/jetstream-editor-tauri/src/lib.rs", "crates/jetstream-editor-tauri/src/commands.rs", "crates/jetstream-editor-tauri/src/bridge_host.rs", "crates/jetstream-editor-tauri/src/surface.rs"].map(jetstreamRead).join("\n");
  excludes(rendererSources, "jetstream:state", "raw state event");
  excludes(rendererSources, "reportViewport", "raw viewport reporter");
  excludes(rendererSources, "set_viewport", "raw viewport invocation");
  for (const endpoint of ["longhorn_bridge_command", "longhorn_bridge_query", "longhorn_bridge_cancel"]) excludes(nativeSources, endpoint, "generic bridge bus");
  excludes(nativeSources, "pub fn set_viewport", "raw viewport endpoint");
  excludes(nativeSources, "std::mem::forget(view)", "native leak");
  excludes(rendererSources, "const SHORTCUTS", "renderer shortcut authority");
  contains(jetstreamRead("editor-ui/src/commands.ts"), "routeCommandKeyboardEvent", "checked keyboard route");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/command_system.rs"), "CommandAdmissionEngine", "checked command authority");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs"), "BackingSurfaceAdapter<SurfaceRuntime>", "checked backing-surface authority");
}

function verifyRetainedAuthorityAndRollback(): void {
  const retained = [
    "crates/jetstream-editor", "crates/jetstream-renderer", "crates/jetstream-world",
    "crates/jetstream-gizmo", "crates/jetstream-platform",
  ];
  equal(git(jetstreamRoot, ["diff", "--name-only", `${fixture.sources.jetstream_previous_authority_commit}..${fixture.sources.jetstream_closeout_commit}`, "--", ...retained]), "", "retained engine authority diff");
  const adapter = jetstreamRead("crates/jetstream-editor-tauri/src/native_content.rs");
  for (const token of ["pending_embedded: Option<Embedded>", "render_thread: Option<JoinHandle<()>>", "crate::surface::attach", "use crate::state::{Shared, ViewportRect}"]) contains(adapter, token, "Jetstream retained adapter");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/commands.rs"), ".input", "semantic input authority");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/surface.rs"), "NativeAttachment", "native-view ownership");
  contains(jetstreamRead("crates/jetstream-editor-tauri/src/scene.rs"), "wgpu::", "consumer WGPU authority");

  assert(fixture.rollback.previous_rust_unit_tests === 19, "previous Rust unit proof drift");
  assert(fixture.rollback.previous_rust_ipc_tests === 4, "previous IPC proof drift");
  assert(fixture.rollback.previous_renderer_tests === 65, "previous renderer proof drift");
  const previousPackage = git(jetstreamRoot, ["show", `${fixture.sources.jetstream_previous_authority_commit}:editor-ui/package.json`]);
  contains(previousPackage, "@poodle/svelte", "previous renderer source");
  const previousHost = git(jetstreamRoot, ["show", `${fixture.sources.jetstream_previous_authority_commit}:crates/jetstream-editor-tauri/src/lib.rs`]);
  contains(previousHost, "generate_handler!", "previous native source");
  for (const failurePath of fixture.validation.aggregate_failure_paths as string[]) {
    equal(git(jetstreamRoot, ["diff", "--name-only", `${fixture.sources.jetstream_previous_authority_commit}..${fixture.sources.jetstream_closeout_commit}`, "--", failurePath]), "", `unrelated failure path ${failurePath}`);
  }
}

function verifyFocusedSuites(): void {
  command("cargo", ["test", "-p", "jetstream-editor-tauri", "--lib", "--test", "ipc"], jetstreamRoot);
  command("effigy", ["editor:test"], jetstreamRoot);
}

function packageManifest(name: string): { peerDependencies?: Record<string, string>; peerDependenciesMeta?: Record<string, Record<string, boolean>> } {
  return JSON.parse(readFileSync(resolve(longhornRoot, `packages/${name}/package.json`), "utf8"));
}
function jetstreamRead(path: string): string { return jetstreamReadRaw(path).toString("utf8"); }
function jetstreamReadRaw(path: string): Buffer { return readFileSync(resolve(jetstreamRoot, path)); }
function sha256(value: Buffer): string { return createHash("sha256").update(value).digest("hex"); }
function git(root: string, args: string[]): string { return command("git", args, root).trim(); }
function command(executable: string, args: string[], cwd: string): string {
  return execFileSync(executable, args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}
function contains(source: string, token: string, label: string): void { assert(source.includes(token), `${label} missing ${JSON.stringify(token)}`); }
function excludes(source: string, token: string, label: string): void { assert(!source.includes(token), `${label} contains ${JSON.stringify(token)}`); }
function equal(actual: unknown, expected: unknown, label: string): void {
  assert(JSON.stringify(actual) === JSON.stringify(expected), `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
}
function assert(condition: unknown, message: string): asserts condition { if (!condition) throw new Error(message); }
