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
      "fixtures/migration/nucleus-card098/project-layout-cutover-v1.json",
    ),
    "utf8",
  ),
) as ProjectLayoutFixture;
const nucleusCommit = fixture.sources.nucleus_cutover_commit;
const longhornCommit = fixture.sources.longhorn_layout_commit;

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
      regions: fixture.shape.regions.map((region) => region.id),
      sizing: fixture.shape.sizing.map((slot) => slot.id),
      layoutDomain: fixture.storage.layout_domain,
      presentationDomain: fixture.storage.presentation_domain,
      removedWorkspaceModules: fixture.workspace_crate_audit.removed_modules,
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
    "longhorn.nucleus-project-layout-cutover.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(fixture.shape.regions.length, 5, "region count");
  assertEqual(fixture.shape.sizing.length, 4, "sizing count");
  assertEqual(fixture.shape.host_binding, "window:primary", "host binding");
  assert(!fixture.shape.surface, "Nucleus shape gained Surface state");
  assertEqual(
    JSON.stringify(fixture.project_scope.new_project_seed),
    JSON.stringify(["agentChat"]),
    "new-project seed",
  );
  assertEqual(
    fixture.project_scope.tasks_policy,
    "one-per-container",
    "Tasks policy",
  );
  assertEqual(fixture.storage.legacy_schemas.length, 10, "legacy schema count");
  assert(fixture.storage.backup_first, "legacy migration is not backup-first");
  assert(fixture.storage.pending_single_layout_claimed_once, "pending claim policy");
  assert(!fixture.storage.dual_write, "layout dual write");
  assert(!fixture.storage.silent_fallback, "legacy fallback");
  assert(fixture.mutation.expected_revision, "expected revision is absent");
  assert(fixture.mutation.stale_preserves_document_and_revision, "stale invariance");
  assert(fixture.mutation.invalid_preserves_document_and_revision, "invalid invariance");
  assertEqual(fixture.mutation.renderer_sizing_debounce_ms, 200, "sizing debounce");
  assert(!fixture.authority.server_layout_write_authority, "server layout authority");
  assert(!fixture.workspace_crate_audit.desktop_layout_authority, "duplicate crate authority");
  assert(!fixture.packages.surface_packages, "Surface package");
  assert(!fixture.packages.package_manager_publication, "package publication");
}

function verifyNucleusCommit(): void {
  assertEqual(
    createHash("sha256").update(nucleusReadRaw("Cargo.lock")).digest("hex"),
    fixture.sources.nucleus_lock_sha256,
    "Nucleus lockfile digest",
  );

  const cargo = nucleusRead("apps/desktop/src-tauri/Cargo.toml");
  for (const dependency of fixture.packages.required) {
    assertContains(cargo, dependency, "Nucleus desktop dependency");
  }
  assertExcludes(cargo, "longhorn-surface", "Nucleus desktop dependency");

  const registry = nucleusRead(
    "apps/desktop/src-tauri/src/workspace_ui/registry.rs",
  );
  for (const region of fixture.shape.regions) {
    assertContains(registry, `region("${region.id}"`, "Nucleus region registry");
  }
  for (const slot of fixture.shape.sizing) {
    assertContains(
      registry,
      `sizing("${slot.id}"`,
      "Nucleus sizing registry",
    );
    assertContains(
      registry,
      slot.default.toString().replace(/\B(?=(\d{3})+(?!\d))/g, "_"),
      "Nucleus sizing default",
    );
  }
  for (const token of [
    "PanelInstancePolicy::OnePerContainer",
    "Sha256Digest::from_bytes(project_id.as_bytes())",
    "identity.push(0)",
    "agent_chat_instance",
  ]) {
    assertContains(registry, token, "Nucleus project registry");
  }

  const runtime = nucleusRead(
    "apps/desktop/src-tauri/src/workspace_ui/runtime.rs",
  );
  for (const command of [
    "CreatePanel",
    "ClosePanel",
    "ActivatePanel",
    "ReorderRegion",
    "MovePanel",
    "SetSizingSlot",
  ]) {
    assertContains(runtime, `LayoutMutationCommand::${command}`, "host mutation adapter");
  }
  for (const token of [
    "requested.layout_revision",
    "stale Nucleus layout revision",
    "PanelPresentationDomain",
    "scope_lock: Mutex<()>"
  ]) {
    assertContains(runtime, token, "Nucleus layout runtime");
  }
  assertExcludes(runtime, "Surface", "Nucleus layout runtime");

  const migration = [
    nucleusRead("apps/desktop/src-tauri/src/workspace_ui/migration.rs"),
    nucleusRead("apps/desktop/src-tauri/src/workspace_ui/legacy.rs"),
  ].join("\n");
  for (const token of [
    "publish_verified_backup",
    "LEGACY_SCHEMA_VERSION",
    "PENDING_PROJECT_SCOPE",
    "complete",
  ]) {
    assertContains(migration, token, "Nucleus layout migration");
  }

  const product = nucleusRead(
    "apps/desktop/src-tauri/src/workspace_ui/product_state.rs",
  );
  for (const field of fixture.authority.nucleus_presentation) {
    assertContains(product, field, "Nucleus product presentation authority");
  }

  const stage = nucleusRead("apps/desktop/src/lib/ProjectWorkspaceStage.svelte");
  for (const token of [
    "workspaceOperationQueue",
    "authoritativeLayoutRevision",
    "latestSaveSequenceByProject",
    "layoutPersistTimer = setTimeout(flushLayoutPersistence, 200)",
    "selectedProject?.project_id === projectId",
  ]) {
    assertContains(stage, token, "Nucleus transition renderer lane");
  }

  const tests = nucleusRead(
    "apps/desktop/src-tauri/src/workspace_ui/tests.rs",
  );
  for (const token of [
    "projects_keep_distinct_layouts_and_new_projects_seed_only_agent_chat",
    "stale_and_invalid_saves_preserve_the_layout_document_and_revision",
    "migration_backs_up_raw_state_and_separates_product_presentations",
    "pending_single_layout_is_claimed_once_then_new_projects_seed_minimally",
    "schemas_one_through_current_split_into_the_same_pending_shape",
    "layout_publication_never_rewrites_the_window_domain",
  ]) {
    assertContains(tests, token, "Nucleus layout evidence test");
  }

  for (const module of fixture.workspace_crate_audit.removed_modules) {
    assert(
      !nucleusFileExists(`crates/nucleus-workspaces/src/${module}.rs`),
      `removed nucleus-workspaces module remains: ${module}`,
    );
  }
  for (const module of fixture.workspace_crate_audit.retained_modules) {
    assert(
      nucleusFileExists(`crates/nucleus-workspaces/src/${module}.rs`),
      `retained nucleus-workspaces module is missing: ${module}`,
    );
  }
  assertExcludes(
    nucleusRead("crates/nucleus-engine/Cargo.toml"),
    "nucleus-workspaces",
    "unused engine dependency",
  );

  const sourceInventory = git(nucleusRoot, [
    "ls-tree",
    "-r",
    "--name-only",
    nucleusCommit,
    "--",
    "apps/desktop/src-tauri/src/workspace_ui",
    "crates/nucleus-workspaces/src",
  ]);
  assertExcludes(sourceInventory.toLowerCase(), "surface", "selected source inventory");
}

function verifyLonghornCommit(): void {
  const conformance = JSON.parse(
    longhornRead("fixtures/layout/nucleus-conformance-v1.json"),
  ) as {
    host_binding: { kind: string; window_id?: string };
    definitions: {
      schema: {
        regions: Array<{ id: string; family_id: string; collapsible: boolean }>;
        sizing_slots: Array<{
          id: string;
          minimum: number;
          default: number;
          maximum: number;
        }>;
      };
      panels: Array<{ id: string; instance_policy: { kind: string } }>;
    };
  };
  assertEqual(conformance.host_binding.kind, "window", "fixture host kind");
  assertEqual(conformance.host_binding.window_id, "window:primary", "fixture window");
  assertEqual(
    JSON.stringify(conformance.definitions.schema.regions),
    JSON.stringify(
      fixture.shape.regions.map((region, order) => ({
        id: region.id,
        family_id: region.family,
        order,
        empty_policy: "hide_when_empty",
        collapsible: region.collapsible,
      })),
    ),
    "Longhorn region conformance",
  );
  assertEqual(
    JSON.stringify(conformance.definitions.schema.sizing_slots),
    JSON.stringify(
      fixture.shape.sizing.map((slot, order) => ({
        id: slot.id,
        order,
        minimum: slot.minimum,
        default: slot.default,
        maximum: slot.maximum,
      })),
    ),
    "Longhorn sizing conformance",
  );
  const tasks = conformance.definitions.panels.find(
    (panel) => panel.id === "panel:tasks",
  );
  assertEqual(
    tasks?.instance_policy.kind,
    "one_per_container",
    "Longhorn Tasks policy",
  );

  const donor = longhornRead(
    "crates/longhorn-layout/tests/layout_model/donors.rs",
  );
  assertContains(donor, "nucleus_five_region_shape_does_not_require_surfaces_or_windows", "donor trace");
  assertContains(donor, "PanelInstancePolicy::OnePerContainer", "donor Tasks policy");
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
  return git(nucleusRoot, [
    "ls-tree",
    "-r",
    "--name-only",
    nucleusCommit,
    "--",
    path,
  ]) === path;
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
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface ProjectLayoutFixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_layout_commit: string;
    readonly nucleus_prior_commit: string;
    readonly nucleus_cutover_commit: string;
    readonly nucleus_lock_sha256: string;
  };
  readonly shape: {
    readonly host_binding: string;
    readonly surface: boolean;
    readonly regions: ReadonlyArray<{
      readonly id: string;
      readonly family: string;
      readonly collapsible: boolean;
    }>;
    readonly sizing: ReadonlyArray<{
      readonly id: string;
      readonly minimum: number;
      readonly default: number;
      readonly maximum: number;
    }>;
  };
  readonly project_scope: {
    readonly new_project_seed: readonly string[];
    readonly tasks_policy: string;
  };
  readonly storage: {
    readonly layout_domain: string;
    readonly presentation_domain: string;
    readonly legacy_schemas: readonly number[];
    readonly backup_first: boolean;
    readonly pending_single_layout_claimed_once: boolean;
    readonly dual_write: boolean;
    readonly silent_fallback: boolean;
  };
  readonly mutation: {
    readonly expected_revision: boolean;
    readonly stale_preserves_document_and_revision: boolean;
    readonly invalid_preserves_document_and_revision: boolean;
    readonly renderer_sizing_debounce_ms: number;
  };
  readonly authority: {
    readonly nucleus_presentation: readonly string[];
    readonly server_layout_write_authority: boolean;
  };
  readonly workspace_crate_audit: {
    readonly removed_modules: readonly string[];
    readonly retained_modules: readonly string[];
    readonly desktop_layout_authority: boolean;
  };
  readonly packages: {
    readonly required: readonly string[];
    readonly surface_packages: boolean;
    readonly package_manager_publication: boolean;
  };
}
