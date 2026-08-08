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
      "fixtures/migration/nucleus-card099/renderer-cutover-v1.json",
    ),
    "utf8",
  ),
) as RendererCutoverFixture;
const nucleusCommit = fixture.sources.nucleus_cutover_commit;
const longhornCommit = fixture.sources.longhorn_poodle_commit;

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
      checkedLifetime: true,
      publicPoodleOnly: true,
      explicitOverlayGeometry: true,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  assertEqual(fixture.schema, "longhorn.nucleus-renderer-cutover.v1", "fixture schema");
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assert(fixture.client.listener_before_snapshot, "listener-first client");
  assert(fixture.client.client_epoch, "client epoch");
  assert(fixture.client.request_keyed_optimism, "request-keyed optimism");
  assert(!fixture.client.whole_snapshot_mutation, "whole-snapshot mutation");
  assert(!fixture.poodle.private_selector_inspection, "private selector inspection");
  assert(!fixture.poodle.private_mime_inspection, "private MIME inspection");
  assert(fixture.overlay.recomputes_on_either_side, "two-sided overlay recomputation");
  assert(fixture.lifecycle.project_switch_isolated, "project-switch isolation");
  assert(fixture.lifecycle.late_mutation_dropped, "late mutation rejection");
  assert(fixture.lifecycle.listener_teardown, "listener teardown");
  assert(fixture.lifecycle.remount_clean, "clean remount");
  assert(fixture.packages.frozen_install, "frozen private install");
  assert(!fixture.packages.duplicate_svelte_runtime, "duplicate Svelte runtime");
  assert(!fixture.packages.surface_packages, "Surface package");
  assert(!fixture.packages.package_manager_publication, "package publication");
}

function verifyNucleusCommit(): void {
  const lock = nucleusReadRaw("apps/desktop/bun.lock");
  assertEqual(
    createHash("sha256").update(lock).digest("hex"),
    fixture.sources.nucleus_renderer_lock_sha256,
    "Nucleus renderer lock digest",
  );

  const packageJson = nucleusRead("apps/desktop/package.json");
  for (const dependency of [
    "@inflatable-cookie/longhorn-core",
    "@inflatable-cookie/longhorn-layout",
    "@inflatable-cookie/longhorn-poodle",
    "@inflatable-cookie/longhorn-svelte",
    "@inflatable-cookie/poodle-styles",
  ]) {
    assertContains(packageJson, `\"${dependency}\": \"file:`, "private renderer graph");
  }
  assertContains(packageJson, "\"overrides\"", "exact private renderer graph");

  const client = nucleusRead("apps/desktop/src/lib/workspaceLayout.ts");
  assertContains(client, "new CheckedSnapshotConnection", "checked layout connection");
  assert(
    client.indexOf("listen: async") < client.indexOf("loadSnapshot:"),
    "layout connection does not declare listener before snapshot",
  );
  for (const token of ["projection_revision", "project_id === projectId", "isNewer:"]) {
    assertContains(client, token, "checked layout client");
  }

  const session = nucleusRead("apps/desktop/src/lib/workspaceLayout.svelte.ts");
  for (const token of [
    "ReactiveClientState",
    "LayoutState",
    "@inflatable-cookie/longhorn-poodle/binding",
    "#generation",
    "#pendingCreates",
    "StaleWorkspaceLayoutEpochError",
    "this.#lifecycle.destroy()",
    "this.layout.destroy()",
  ]) {
    assertContains(session, token, "per-project layout session");
  }

  const stage = nucleusRead("apps/desktop/src/lib/ProjectWorkspaceStage.svelte");
  for (const token of ["LayoutDockRegion", "LayoutSplitView", "WorkspaceLayoutSession"]) {
    assertContains(stage, token, "public renderer composition");
  }
  for (const token of [
    "querySelector",
    "application/x-poodle",
    "data-poodle-tab-id",
    "saveWorkspaceUi",
    "loadWorkspaceUi",
  ]) {
    assertExcludes(stage, token, "Nucleus workspace renderer");
  }
  assert(!nucleusFileExists("apps/desktop/src/lib/workspaceUi.ts"), "old workspace authority remains");

  const overlays = [
    nucleusRead("apps/desktop/src/App.svelte"),
    nucleusRead("apps/desktop/src/lib/nativePanelVisibility.ts"),
    nucleusRead("apps/desktop/src/lib/BrowserPanel.svelte"),
  ].join("\n");
  for (const token of [
    "onSurfaceGeometryChange",
    "OverlaySurfaceGeometryChange",
    "setNativeBrowserViewportGeometry",
  ]) {
    assertContains(overlays, token, "explicit overlay geometry");
  }
  assertExcludes(overlays, "querySelector", "overlay adapter");
  assertExcludes(overlays, "data-native-browser-viewport", "Browser geometry adapter");

  const host = nucleusRead("apps/desktop/src-tauri/src/lib.rs");
  for (const command of [
    "workspace_layout_snapshot",
    "prepare_workspace_panel",
    "mutate_workspace_layout",
    "update_workspace_panel_presentation",
  ]) {
    assertContains(host, command, "Nucleus layout host");
  }
  assertExcludes(host, "load_workspace_ui_config", "Nucleus layout host");
  assertExcludes(host, "save_workspace_ui_config", "Nucleus layout host");

  const mounted = nucleusRead("apps/desktop/src/lib/workspaceLayoutSession.vitest.ts");
  for (const token of [
    "isolates mounted project switches",
    "drops a late mutation result",
    "disposed).toBe(true)",
    "project:gamma",
  ]) {
    assertContains(mounted, token, "mounted lifecycle evidence");
  }
}

function verifyLonghornCommit(): void {
  const metadata = JSON.parse(longhornRead("packages/poodle/package.json"));
  assertEqual(
    metadata.exports["./binding"].import,
    "./src/binding.ts",
    "state-only binding export",
  );
  const split = longhornRead("packages/poodle/src/LayoutSplitView.svelte");
  for (const token of [
    "primaryHidden",
    "secondaryHidden",
    "primaryCollapsedSize={primaryHidden ? 0 : null}",
    "disabled={primaryHidden || secondaryHidden}",
  ]) {
    assertContains(split, token, "public split projection");
  }
  const dock = longhornRead("packages/poodle/src/LayoutDockRegion.svelte");
  for (const token of ["sizeRole", "density", "{size}"]) {
    assertContains(dock, token, "public dock presentation");
  }
  const tests = [
    longhornRead("packages/poodle/tests/split.test.ts"),
    longhornRead("packages/poodle/tests/package.test.ts"),
  ].join("\n");
  assertContains(tests, "hidden pane without dispatching durable collapse", "hidden-pane proof");
  assertContains(tests, "metadata.exports[\"./binding\"]", "binding export proof");
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

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
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
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
  }
}

interface RendererCutoverFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_poodle_commit: string;
    readonly nucleus_prior_commit: string;
    readonly nucleus_cutover_commit: string;
    readonly nucleus_renderer_lock_sha256: string;
  };
  readonly client: Record<string, boolean | string>;
  readonly poodle: {
    readonly private_selector_inspection: boolean;
    readonly private_mime_inspection: boolean;
  };
  readonly overlay: { readonly recomputes_on_either_side: boolean };
  readonly lifecycle: Record<string, boolean>;
  readonly packages: Record<string, boolean>;
}
