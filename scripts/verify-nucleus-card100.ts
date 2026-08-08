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
      "fixtures/migration/nucleus-card100/native-browser-cutover-v1.json",
    ),
    "utf8",
  ),
) as NativeBrowserCutoverFixture;
const nucleusCommit = fixture.sources.nucleus_cutover_commit;
const longhornCommit = fixture.sources.longhorn_policy_commit;

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
      checkedNativeContent: true,
      consumerPolicyRetained: true,
      remoteCapability: false,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.nucleus-native-browser-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assert(fixture.identity.one_current_child_per_panel, "one current child");
  assert(fixture.identity.generation_advances_after_destroy, "generation advance");
  assert(fixture.protocol.listener_before_snapshot, "listener-first protocol");
  assert(fixture.protocol.checked_client, "checked native-content client");
  assert(!fixture.protocol.product_payloads_shared, "shared product payload");
  assert(!fixture.protocol.raw_renderer_webview_api, "raw renderer Webview API");
  assert(fixture.geometry.exact_consumer_viewport, "exact viewport");
  assert(fixture.geometry.explicit_device_scale, "explicit scale");
  assert(fixture.lifecycle.tab_switch_hides, "tab-switch hide");
  assert(fixture.lifecycle.remount_reuses_generation, "remount reuse");
  assert(fixture.lifecycle.panel_close_destroys, "panel close destroy");
  assertEqual(fixture.policy.popup, "deny-and-notice", "popup policy");
  assertEqual(fixture.policy.download, "deny-and-notice", "download policy");
  assertEqual(fixture.policy.data_store_identifier, null, "data-store policy");
  assert(!fixture.policy.persisted_cookies_or_credentials, "persisted browser secrets");
  assert(!fixture.capability.remote_match, "remote capability match");
  assert(!fixture.capability.raw_webview_permissions, "raw Webview permissions");
  assert(!fixture.capability.remote_child_tauri_capability, "remote child capability");
  assert(fixture.packages.frozen_install, "frozen private install");
  assert(!fixture.packages.duplicate_svelte_runtime, "duplicate Svelte runtime");
  assert(!fixture.packages.surface_packages, "Surface package");
  assert(!fixture.packages.package_manager_publication, "package publication");
  assert(fixture.packaged_macos_smoke.example_domain_rendered, "packaged child render");
  assertEqual(fixture.packaged_macos_smoke.reuse_generation, 1, "reuse generation");
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
    "@inflatable-cookie/longhorn-native-content",
    "@inflatable-cookie/longhorn-native-content-svelte",
  ]) {
    assertContains(packageJson, `\"${dependency}\": \"file:`, "private native-content graph");
  }

  const rendererClient = nucleusRead("apps/desktop/src/lib/browserPanel.ts");
  for (const token of [
    "new NativeContentClient",
    "createTauriNativeContentPort",
    "island:nucleus-browser:",
    "request:nucleus-browser:",
  ]) {
    assertContains(rendererClient, token, "checked Browser client");
  }
  assertExcludes(rendererClient, "@tauri-apps/api/webview", "Browser renderer client");
  assertExcludes(rendererClient, "new Webview", "Browser renderer client");

  const panel = nucleusRead("apps/desktop/src/lib/BrowserPanel.svelte");
  for (const token of [
    "new NativeContentSession",
    "use:nativeContentViewport={session}",
    "globalThis.devicePixelRatio",
    "resolveNativeContentVisibility",
    "nucleus:unmounted",
    "nucleus:inactive-panel",
    "nucleus:empty-viewport",
    "nucleus:workspace-gesture",
    "nucleus:overlay",
    "setNativeBrowserViewportGeometry",
  ]) {
    assertContains(panel, token, "Browser Svelte session");
  }
  assertExcludes(panel, "@tauri-apps/api/webview", "Browser Svelte session");

  const stage = nucleusRead("apps/desktop/src/lib/ProjectWorkspaceStage.svelte");
  assertContains(stage, "destroyBrowserIsland", "Browser panel close");
  assertContains(stage, "panelIsVisible", "active Browser visibility");

  const host = nucleusRead("apps/desktop/src-tauri/src/browser_panel.rs");
  for (const token of [
    "NativeContentProtocolHost",
    "ChildViewAdapter",
    "TauriChildViewRuntime",
    "ChildViewPolicyHooks::new",
    "ChildViewPolicyEvent::PopupDenied",
    "ChildViewPolicyEvent::DownloadDenied",
    "AttachGeneration::INITIAL",
    "value.checked_next()",
    "longhorn_native_content_connect",
    "longhorn_native_content_snapshot",
    "longhorn_native_content_update_desired",
    "longhorn_native_content_decide_size",
    "browser_panel_hide_for_unmount",
    "browser_panel_destroy",
    "is_supported_http_url",
    "None,",
    "CURSOR_BRIDGE_SCRIPT",
  ]) {
    assertContains(host, token, "Nucleus native Browser host");
  }
  for (const token of ["browser_panel_ensure", "browser_panel_set_bounds"] ) {
    assertExcludes(host, token, "superseded Browser host");
  }

  const appHost = nucleusRead("apps/desktop/src-tauri/src/lib.rs");
  assertContains(appHost, "browser_panel::install(app)", "Browser runtime registration");
  assertContains(appHost, ".teardown()", "Browser host teardown");

  const config = JSON.parse(nucleusRead("apps/desktop/src-tauri/tauri.conf.json"));
  const capability = config.app.security.capabilities[0];
  assertEqual(capability.webviews, ["main"], "controller capability webviews");
  assert(!("remote" in capability), "remote capability selector exists");
  assert(
    capability.permissions.every((permission: string) => !permission.includes("webview")),
    "raw Webview permission remains",
  );
}

function verifyLonghornCommit(): void {
  const policy = longhornRead(
    "crates/longhorn-tauri-native-content-child-view/src/policy.rs",
  );
  for (const token of [
    "pub enum ChildViewPolicyEvent",
    "pub struct ChildViewPolicyHooks",
    "MAX_INITIALIZATION_SCRIPT_BYTES: usize = 64 * 1024",
    "script.contains('\\0')",
    "policy_hooks: ChildViewPolicyHooks",
  ]) {
    assertContains(policy, token, "shared child policy seam");
  }

  const runtime = longhornRead(
    "crates/longhorn-tauri-native-content-child-view/src/tauri_runtime.rs",
  );
  for (const token of [
    ".initialization_script(script)",
    "ChildViewPolicyEvent::PageLoadStarted",
    "ChildViewPolicyEvent::PageLoadFinished",
    "ChildViewPolicyEvent::PopupDenied",
    "ChildViewPolicyEvent::DownloadDenied",
    "NewWindowResponse::Deny",
    "false",
  ]) {
    assertContains(runtime, token, "shared child runtime policy");
  }

  const readme = longhornRead(
    "crates/longhorn-tauri-native-content-child-view/README.md",
  );
  assertContains(readme, "never enters the renderer protocol", "native-only hook boundary");
  assertContains(readme, "Remote content receives no capabilities", "remote capability boundary");
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
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface NativeBrowserCutoverFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_policy_commit: string;
    readonly nucleus_prior_commit: string;
    readonly nucleus_cutover_commit: string;
    readonly nucleus_renderer_lock_sha256: string;
  };
  readonly identity: Record<string, boolean | string>;
  readonly protocol: Record<string, boolean>;
  readonly geometry: Record<string, boolean | string | string[]>;
  readonly lifecycle: Record<string, boolean>;
  readonly policy: Record<string, boolean | string | string[] | null>;
  readonly capability: Record<string, boolean | string[]>;
  readonly packages: Record<string, boolean>;
  readonly packaged_macos_smoke: Record<string, boolean | number | string[]>;
}
