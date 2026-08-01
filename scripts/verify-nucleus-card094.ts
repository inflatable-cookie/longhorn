import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const fixturePath = resolve(
  longhornRoot,
  "fixtures/migration/nucleus-card094/behavior-freeze-v1.json",
);
const fixture = JSON.parse(readFileSync(fixturePath, "utf8")) as FreezeFixture;
const repositories = {
  nucleus: resolve(
    process.env.NUCLEUS_REPO ?? resolve(longhornRoot, "../nucleus"),
  ),
  poodle: resolve(
    process.env.POODLE_REPO ?? resolve(longhornRoot, "../poodle"),
  ),
} as const;

assertEqual(
  fixture.schema,
  "longhorn.nucleus-migration-freeze.v1",
  "fixture schema",
);
assertEqual(fixture.outcome, "pass", "fixture outcome");

for (const [name, expected] of Object.entries(fixture.donors) as Array<
  [RepositoryName, DonorFixture]
>) {
  const root = repositories[name];
  git(root, ["cat-file", "-e", `${expected.commit}^{commit}`]);
  const clean = git(root, ["status", "--porcelain"]) === "";
  assertEqual(clean, expected.expected_clean, `${name} worktree state`);
}

for (const check of fixture.source_checks) {
  const donor = fixture.donors[check.repository];
  const source = gitRead(
    repositories[check.repository],
    donor.commit,
    check.path,
  );
  for (const token of check.contains) {
    assert(
      source.includes(token),
      `${check.repository}/${check.path} lost ${JSON.stringify(token)}`,
    );
  }
  for (const token of check.excludes) {
    assert(
      !source.includes(token),
      `${check.repository}/${check.path} gained ${JSON.stringify(token)}`,
    );
  }
}

verifyNucleusConfiguration();
verifyLonghornLayoutMapping();
verifyFrozenSemantics();

console.log(
  JSON.stringify(
    {
      schema: "longhorn.nucleus-migration-freeze-verification.v1",
      outcome: fixture.outcome,
      donorCommits: {
        nucleus: fixture.donors.nucleus.commit,
        poodle: fixture.donors.poodle.commit,
      },
      sourceChecks: fixture.source_checks.length,
      hierarchy: fixture.layout.hierarchy,
      regions: fixture.layout.regions,
      storageTarget: {
        profile: fixture.storage.target_profile,
        leaf: fixture.storage.target_leaf,
      },
      browserPolicy: {
        schemes: fixture.browser.schemes,
        popup: fixture.browser.popup,
        download: fixture.browser.download,
        dataStore: fixture.browser.data_store,
        remoteCapabilities: fixture.browser.remote_child_tauri_capabilities,
      },
      poodleSeam: fixture.poodle_seam.status,
      donorWritesAdmitted: fixture.admission.donor_writes,
      gates: fixture.admission.gates,
    },
    null,
    2,
  ),
);

function verifyNucleusConfiguration(): void {
  const donor = fixture.donors.nucleus;
  const tauri = JSON.parse(
    gitRead(
      repositories.nucleus,
      donor.commit,
      "apps/desktop/src-tauri/tauri.conf.json",
    ),
  ) as {
    identifier: string;
    app: {
      windows: Array<{ label: string; visible: boolean }>;
      security: {
        capabilities: Array<{
          identifier: string;
          webviews: string[];
          permissions: string[];
        }>;
      };
    };
  };
  assertEqual(
    tauri.identifier,
    fixture.donors.nucleus.app_id,
    "Nucleus app id",
  );
  assertEqual(tauri.app.windows[0]?.label, "main", "primary window label");
  assertEqual(
    tauri.app.windows[0]?.visible,
    false,
    "primary window hidden startup",
  );
  const main = tauri.app.security.capabilities.find(
    (capability) => capability.identifier === "main-window",
  );
  assert(main !== undefined, "main-window capability is missing");
  assertEqual(
    JSON.stringify(main.webviews),
    JSON.stringify(["main"]),
    "trusted webviews",
  );
  assert(
    !main.webviews.some(
      (label) => label.includes("nucleus-browser") || label.includes("*"),
    ),
    "remote Browser child entered the trusted capability",
  );

  const desktopManifest = JSON.parse(
    gitRead(
      repositories.nucleus,
      donor.commit,
      "apps/desktop/package.json",
    ),
  ) as { dependencies: Record<string, string> };
  assert(
    Object.keys(desktopManifest.dependencies).every(
      (name) => !name.startsWith("@longhorn/"),
    ),
    "Nucleus already resolves a Longhorn renderer dependency",
  );
  for (const name of [
    "@poodle/headless",
    "@poodle/icons-lucide",
    "@poodle/svelte",
    "@poodle/svelte-tokens",
  ]) {
    assert(
      desktopManifest.dependencies[name]?.startsWith("file:../../../poodle/"),
      `${name} is no longer a sibling source dependency at the freeze`,
    );
  }
}

function verifyLonghornLayoutMapping(): void {
  const path = resolve(longhornRoot, fixture.layout.longhorn_fixture);
  const source = readFileSync(path, "utf8");
  const longhornFixture = JSON.parse(source) as {
    host_binding: { kind: string };
    definitions: { schema: { regions: unknown[]; sizing_slots: unknown[] } };
  };
  assertEqual(
    longhornFixture.host_binding.kind,
    "window",
    "Longhorn Nucleus host binding",
  );
  assertEqual(
    longhornFixture.definitions.schema.regions.length,
    5,
    "Longhorn region count",
  );
  assertEqual(
    longhornFixture.definitions.schema.sizing_slots.length,
    4,
    "Longhorn sizing count",
  );
  assert(
    !source.toLowerCase().includes("surface"),
    "Longhorn Nucleus fixture gained Surface state",
  );
  assertEqual(
    fixture.layout.longhorn_fixture_mapping,
    "compatible-shape-not-literal-current-donor",
    "Longhorn fixture mapping status",
  );
}

function verifyFrozenSemantics(): void {
  assertEqual(
    JSON.stringify(fixture.layout.hierarchy),
    JSON.stringify(["display", "window", "region", "panel"]),
    "Nucleus hierarchy",
  );
  assertEqual(fixture.layout.surface_dependency, false, "Surface dependency");
  assertEqual(fixture.layout.regions.length, 5, "current donor region count");
  assertEqual(
    fixture.layout.sizing_slots.length,
    4,
    "current donor sizing count",
  );
  assertEqual(
    fixture.layout.resource_targets_shared,
    false,
    "resource authority",
  );
  assertEqual(
    fixture.storage.target_profile,
    "platform-native-v1",
    "storage profile",
  );
  assertEqual(
    fixture.storage.target_leaf,
    "com.inflatablecookie.nucleus",
    "storage leaf",
  );
  assertEqual(fixture.storage.stable_storage_name, null, "stable storage name");
  assertEqual(fixture.storage.locator_commit, "last", "locator commit order");
  assertEqual(
    fixture.storage.legacy_source_retained,
    true,
    "legacy source retention",
  );
  assertEqual(fixture.storage.dual_write, false, "storage dual write");
  assertEqual(fixture.storage.silent_fallback, false, "storage fallback");
  assertEqual(
    fixture.storage.stores.find((store) =>
      store.source.endsWith("nucleus.sqlite"),
    )?.adapter,
    "sqlite-native-snapshot",
    "SQLite migration adapter",
  );
  assertEqual(
    fixture.storage.stores.find((store) =>
      store.source.endsWith("task-review-snapshots"),
    )?.target_class,
    "workspace-local",
    "snapshot storage class",
  );
  assertEqual(
    fixture.storage.stores.find((store) =>
      store.source.endsWith("task-review-snapshots"),
    )?.adapter,
    "nucleus-snapshot-tree",
    "snapshot transition adapter",
  );
  assertEqual(
    fixture.storage.stores.find((store) =>
      store.source.endsWith("task-review-snapshots"),
    )?.backup,
    "excluded-by-nucleus-retention-policy",
    "snapshot backup policy",
  );
  assertEqual(
    fixture.storage.stores.find((store) =>
      store.source.endsWith("task-review-snapshots"),
    )?.retention,
    "active-or-awaiting-review-then-seven-day-cleanup-grace",
    "snapshot retention policy",
  );

  assertEqual(
    JSON.stringify(fixture.browser.schemes),
    JSON.stringify(["http", "https"]),
    "schemes",
  );
  assertEqual(
    fixture.browser.popup,
    "deny-with-trusted-notice",
    "popup policy",
  );
  assertEqual(
    fixture.browser.download,
    "deny-with-trusted-notice",
    "download policy",
  );
  assertEqual(
    fixture.browser.data_store,
    "platform-normal-shared-store",
    "data-store policy",
  );
  assertEqual(
    fixture.browser.explicit_macos_data_store_id,
    null,
    "macOS data-store id",
  );
  assertEqual(
    fixture.browser.remote_child_tauri_capabilities,
    false,
    "remote capability",
  );
  assertEqual(
    fixture.browser.mount_order,
    "listener-before-ensure",
    "Browser mount order",
  );
  assertEqual(fixture.browser.panel_close, "destroy", "Browser close policy");
  assertEqual(
    fixture.browser.unmount,
    "hide-for-process-reuse",
    "Browser unmount policy",
  );

  assertEqual(
    fixture.poodle_seam.public_open_change,
    true,
    "Poodle open state seam",
  );
  assertEqual(
    fixture.poodle_seam.public_custom_surface_anchoring,
    true,
    "Poodle custom anchored seam",
  );
  assertEqual(
    fixture.poodle_seam.public_builtin_surface_geometry,
    false,
    "Poodle built-in overlay geometry seam",
  );
  assertEqual(
    fixture.poodle_seam.status,
    "missing-required-public-overlay-geometry-seam",
    "Poodle seam gate",
  );
  assertEqual(
    fixture.poodle_seam.current_measurement_defect,
    "live-portalled-surface-structurally-unreachable-from-query-root",
    "Poodle portal/query defect",
  );
  assertEqual(
    fixture.poodle_seam.migration_policy,
    "restore-intended-exact-intersection-not-broken-query",
    "Poodle seam migration policy",
  );
  assertEqual(fixture.admission.donor_writes, true, "donor-write admission");
  assertEqual(fixture.admission.gates.length, 0, "remaining admission gates");
  assert(
    fixture.rollback_slices.every((slice) => slice.dual_write === false),
    "a rollback slice permits dual write",
  );
}

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
  }).trim();
}

function gitRead(root: string, commit: string, path: string): string {
  return git(root, ["show", `${commit}:${path}`]);
}

function assert(condition: boolean, message: string): asserts condition {
  if (!condition) {
    throw new Error(message);
  }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (actual !== expected) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

type RepositoryName = "nucleus" | "poodle";

interface DonorFixture {
  readonly repository: string;
  readonly branch: string;
  readonly commit: string;
  readonly expected_clean: boolean;
  readonly app_id?: string;
}

interface FreezeFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly donors: Record<RepositoryName, DonorFixture>;
  readonly source_checks: readonly {
    readonly repository: RepositoryName;
    readonly path: string;
    readonly contains: readonly string[];
    readonly excludes: readonly string[];
  }[];
  readonly storage: {
    readonly target_profile: string;
    readonly target_leaf: string;
    readonly stable_storage_name: string | null;
    readonly locator_commit: string;
    readonly legacy_source_retained: boolean;
    readonly dual_write: boolean;
    readonly silent_fallback: boolean;
    readonly stores: readonly {
      readonly source: string;
      readonly target_class: string;
      readonly adapter: string;
      readonly backup?: string;
      readonly retention?: string;
    }[];
  };
  readonly layout: {
    readonly hierarchy: readonly string[];
    readonly surface_dependency: boolean;
    readonly regions: readonly string[];
    readonly sizing_slots: readonly string[];
    readonly resource_targets_shared: boolean;
    readonly longhorn_fixture: string;
    readonly longhorn_fixture_mapping: string;
  };
  readonly browser: {
    readonly schemes: readonly string[];
    readonly popup: string;
    readonly download: string;
    readonly data_store: string;
    readonly explicit_macos_data_store_id: string | null;
    readonly remote_child_tauri_capabilities: boolean;
    readonly mount_order: string;
    readonly panel_close: string;
    readonly unmount: string;
  };
  readonly poodle_seam: {
    readonly public_open_change: boolean;
    readonly public_custom_surface_anchoring: boolean;
    readonly public_builtin_surface_geometry: boolean;
    readonly status: string;
  };
  readonly rollback_slices: readonly { readonly dual_write: boolean }[];
  readonly admission: {
    readonly donor_writes: boolean;
    readonly gates: readonly string[];
  };
}
