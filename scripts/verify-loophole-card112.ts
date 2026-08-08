import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const loopholeRoot = resolve(
  process.env.LOOPHOLE_REPO ?? resolve(longhornRoot, "../loophole"),
);
const poodleRoot = resolve(longhornRoot, "../poodle");
const signalRoot = resolve(longhornRoot, "../signal");
const soundcheckRoot = resolve(longhornRoot, "../soundcheck-library");
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/loophole-card112/conformance-closeout-v1.json",
    ),
    "utf8",
  ),
) as Fixture;

verifyFixture();
verifyReceipts();
verifyPriorCards();
const inventory = verifyComposition();
const matrices = verifyMatrices();
const duplicates = verifyDuplicateAndAdapterPosture();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: fixture.outcome,
      sources: fixture.sources,
      hierarchy: fixture.composition.hierarchy,
      inventory,
      matrices,
      duplicates,
      retainedAdapters: fixture.retained_adapters,
      deferred: fixture.deferred,
      recovery: fixture.recovery,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  equal(
    fixture.schema,
    "longhorn.loophole-migration-conformance-closeout.v1",
    "fixture schema",
  );
  equal(fixture.outcome, "pass", "fixture outcome");
  equal(
    fixture.composition.hierarchy,
    ["display", "window", "surface", "region", "panel"],
    "hierarchy",
  );
  assert(
    !fixture.composition.package_manager_publication,
    "package publication",
  );
  assert(
    !fixture.composition.fork_tree_packages,
    "fork-tree packages admitted",
  );
  assert(
    fixture.composition.excluded_later_systems,
    "later systems are not explicitly excluded",
  );
  assert(
    !fixture.recovery.dual_write_authority,
    "rollback depends on dual authority",
  );
  assert(
    !fixture.recovery.silent_fallback,
    "rollback depends on silent fallback",
  );
  assert(
    !fixture.recovery.automatic_source_cleanup,
    "legacy source cleanup was automatic",
  );
  assert(
    fixture.recovery.cleanup_requires_exact_receipt_and_operator_authority,
    "cleanup is not receipt and operator bound",
  );
}

function verifyReceipts(): void {
  for (const [label, root, expected] of [
    ["Loophole", loopholeRoot, fixture.sources.loophole_commit],
    ["Poodle", poodleRoot, fixture.sources.poodle_commit],
    ["Signal", signalRoot, fixture.sources.signal_commit],
    ["Soundcheck", soundcheckRoot, fixture.sources.soundcheck_commit],
  ] as const) {
    equal(git(root, ["rev-parse", "HEAD"]), expected, `${label} HEAD`);
    assertClean(label, root);
  }
  git(longhornRoot, [
    "cat-file",
    "-e",
    `${fixture.sources.longhorn_selected_source_commit}^{commit}`,
  ]);
  const selected = [
    "Cargo.toml",
    "Cargo.lock",
    "bun.lock",
    "crates/longhorn-core",
    "crates/longhorn-config",
    "crates/longhorn-tauri-config",
    "crates/longhorn-display",
    "crates/longhorn-windowing",
    "crates/longhorn-tauri-windowing",
    "crates/longhorn-windowing-config",
    "crates/longhorn-layout",
    "crates/longhorn-layout-config",
    "crates/longhorn-surfaces",
    "crates/longhorn-surfaces-config",
    "crates/longhorn-surface-windowing",
    "crates/longhorn-transfer",
    "crates/longhorn-surface-transfer",
    "crates/longhorn-tauri-transfer",
    "crates/longhorn-settings",
    "crates/longhorn-settings-config",
    "crates/longhorn-tauri-settings",
    "crates/longhorn-command",
    "crates/longhorn-command-config",
    "crates/longhorn-command-settings",
    "crates/longhorn-tauri-command",
    "crates/longhorn-history",
    "crates/longhorn-tauri-history",
    "packages/core",
    "packages/config",
    "packages/layout",
    "packages/surfaces",
    "packages/transfer",
    "packages/surface-transfer",
    "packages/settings",
    "packages/commands",
    "packages/history",
    "packages/tauri",
    "packages/svelte",
    "packages/poodle",
  ];
  const diff = spawnSync(
    "git",
    [
      "diff",
      "--quiet",
      fixture.sources.longhorn_selected_source_commit,
      "HEAD",
      "--",
      ...selected,
    ],
    { cwd: longhornRoot, stdio: "ignore" },
  );
  assert(
    diff.status === 0,
    "selected Longhorn source changed after artifact proof",
  );
  const dirty = git(longhornRoot, ["status", "--porcelain", "--", ...selected]);
  assert(!dirty, `selected Longhorn source is dirty:\n${dirty}`);
}

function verifyPriorCards(): void {
  for (const card of ["102", "110", "111"]) {
    const proof = JSON.parse(
      command(longhornRoot, ["bun", `scripts/verify-loophole-card${card}.ts`]),
    );
    assert(
      String(proof.outcome).startsWith("pass"),
      `Card ${card} evidence does not pass`,
    );
  }
}

function verifyComposition(): Record<string, unknown> {
  const manifest = JSON.parse(source("aura/package.json")) as {
    dependencies: Record<string, string>;
  };
  const rendererPackages = [
    "@inflatable-cookie/longhorn-core",
    "@inflatable-cookie/longhorn-config",
    "@inflatable-cookie/longhorn-layout",
    "@inflatable-cookie/longhorn-surfaces",
    "@inflatable-cookie/longhorn-transfer",
    "@inflatable-cookie/longhorn-surface-transfer",
    "@inflatable-cookie/longhorn-settings",
    "@inflatable-cookie/longhorn-commands",
    "@inflatable-cookie/longhorn-history",
    "@inflatable-cookie/longhorn-tauri",
    "@inflatable-cookie/longhorn-svelte",
    "@inflatable-cookie/longhorn-poodle",
  ];
  for (const name of rendererPackages) {
    assert(
      manifest.dependencies[name]?.startsWith("file:../../longhorn/"),
      `${name} is not admitted from private Longhorn source`,
    );
  }
  const allManifests = command(
    loopholeRoot,
    [
      "rg",
      "-n",
      "history-tree|native-content|longhorn-bridge|longhorn-operation|longhorn-notifications",
      "--glob",
      "Cargo.toml",
      "--glob",
      "package.json",
      "--glob",
      "!**/reference/**",
    ],
    true,
  );
  assert(!allManifests, `deferred package entered graph:\n${allManifests}`);

  const tauri = JSON.parse(source("aura/src-tauri/tauri.conf.json"));
  equal(tauri.identifier, fixture.composition.canonical_app_id, "Tauri id");
  const capability = tauri.app.security.capabilities[0];
  equal(capability.identifier, fixture.capability.identifier, "capability id");
  equal(capability.windows, fixture.capability.windows, "capability windows");
  assert(!("remote" in capability), "remote capability selector exists");
  assert(
    !capability.windows.some((window: string) => window.includes("plugin")),
    "plugin GUI entered workspace capability",
  );

  return {
    rendererPackages,
    canonicalAppId: tauri.identifier,
    capability: capability.identifier,
    capabilityWindows: capability.windows,
    remoteCapability: false,
    fullSurfaceHierarchy: true,
    laterPackagesAbsent: true,
  };
}

function verifyMatrices(): Record<string, number> {
  const groups: Record<string, readonly [string, string][]> = {
    storage: [
      [
        "aura/src-tauri/src/storage_migration.rs",
        "established_root_is_reclassified_without_copying_itself",
      ],
      [
        "aura/src-tauri/src/storage_migration.rs",
        "legacy_tauri_root_imports_only_when_product_root_is_empty",
      ],
      [
        "aura/src-tauri/src/window_host/mod.rs",
        "interrupted_import_can_publish_receipt_from_retained_sources",
      ],
    ],
    hierarchy: [
      [
        "aura/src-tauri/src/window_host/mod.rs",
        "restart_and_display_loss_return_preserve_home_and_per_display_geometry",
      ],
      [
        "aura/src-tauri/src/services/layout_host.rs",
        "literal_catalogue_and_eight_region_schema_validate",
      ],
      [
        "aura/src-tauri/src/services/surface_host.rs",
        "focused_habitat_stays_outside_registered_document",
      ],
      [
        "aura/src-tauri/src/services/shell.rs",
        "moving_any_windows_last_surface_out_disables_the_emptied_window",
      ],
      [
        "aura/src-tauri/src/services/shell.rs",
        "spawn_window_for_surface_creates_a_window_on_the_chosen_display",
      ],
      [
        "aura/src-tauri/src/services/shell.rs",
        "topology_persistence_failure_restores_both_registered_authorities",
      ],
    ],
    shell: [
      [
        "aura/src-tauri/src/services/settings_host.rs",
        "registry_composes_product_commands_and_real_storage_diagnostics_only",
      ],
      [
        "aura/src-tauri/src/services/command_host.rs",
        "legacy_last_override_wins_and_disables_shadowed_base_binding",
      ],
      [
        "aura/src-tauri/src/services/history_host.rs",
        "protocol_navigation_commits_once_and_rejects_the_stale_revision",
      ],
    ],
    recovery: [
      ["pulse/src/tests.rs", "history_roundtrips_through_project_manifest"],
      [
        "pulse/src/tests.rs",
        "session_journal_crash_recovery_replays_edits_past_autosave",
      ],
      [
        "pulse/src/tests.rs",
        "session_journal_clean_save_reopen_keeps_cross_session_undo",
      ],
      [
        "pulse/src/tests.rs",
        "checkpoint_journal_interleave_crash_sim_pins_replay_outcomes",
      ],
      [
        "pulse/src/tests.rs",
        "failed_multi_entry_history_apply_restores_product_and_structural_state_exactly",
      ],
    ],
  };
  const counts: Record<string, number> = {};
  for (const [group, checks] of Object.entries(groups)) {
    for (const [path, token] of checks) {
      assert(
        source(path).includes(`fn ${token}`),
        `${group} matrix lost ${token}`,
      );
    }
    counts[group] = checks.length;
  }
  return counts;
}

function verifyDuplicateAndAdapterPosture(): Record<string, unknown> {
  const removedPaths = [
    "aura/src-tauri/src/services/window_apply.rs",
    "aura/src-tauri/src/adapters/keyboard.rs",
    "aura/src/renderer/host-provider/keyboard-host-provider.ts",
    "aura/src/renderer/settings/chord-capture.ts",
    "aura/src/renderer/settings/keybinding-conflicts.ts",
    "aura/src/renderer/settings/keymap-overrides-wire.ts",
    "aura/src/renderer/workspace/history-entries.ts",
  ];
  for (const path of removedPaths) {
    assert(
      !existsSync(resolve(loopholeRoot, path)),
      `superseded path remains: ${path}`,
    );
  }
  const productionRoots = ["aura/src", "aura/src-tauri/src"];
  for (const token of [
    "WindowApplyCoordinator",
    "application/x-poodle",
    "data-poodle-tab-id",
    "PoodleTabDrag",
  ]) {
    const matches = command(
      loopholeRoot,
      [
        "rg",
        "-n",
        "-F",
        token,
        ...productionRoots,
        "--glob",
        "!**/reference/**",
        "--glob",
        "!**/tests/**",
      ],
      true,
    );
    assert(
      !matches,
      `superseded active mechanism remains: ${token}\n${matches}`,
    );
  }
  const requiredAdapters = [
    ["aura/src-tauri/src/services/layout_host.rs", "replace_from_legacy"],
    ["aura/src-tauri/src/services/surface_host.rs", "replace_from_legacy"],
    ["aura/src-tauri/src/services/shell.rs", "SurfaceHabitat::FocusedPanel"],
    [
      "aura/src/renderer/workspace/workspace-transfer.ts",
      "empty_display_disabled",
    ],
    [
      "aura/src/renderer/workspace/workspace-extended-input-adapter.ts",
      "LoopholeExtendedInputAdapter",
    ],
    [
      "aura/src/renderer/workspace/HistoryPanel.svelte",
      "historySession.refresh()",
    ],
    ["aura/src-tauri/src/commands/mod.rs", "pulse::aura_jump_to_history_entry"],
  ] as const;
  for (const [path, token] of requiredAdapters) {
    assert(
      source(path).includes(token),
      `retained adapter lost: ${path} ${token}`,
    );
  }
  return {
    supersededPathsAbsent: removedPaths.length,
    supersededTokensAbsent: 4,
    classifiedAdaptersPresent: requiredAdapters.length,
    genericAuthoritiesPerMigratedConcern: 1,
  };
}

function source(path: string): string {
  return readFileSync(resolve(loopholeRoot, path), "utf8");
}

function assertClean(label: string, root: string): void {
  const status = git(root, ["status", "--porcelain"]);
  assert(!status, `${label} worktree is dirty:\n${status}`);
}

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function command(
  root: string,
  arguments_: string[],
  allowNoMatches = false,
): string {
  const result = spawnSync(arguments_[0], arguments_.slice(1), {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 20 * 1024 * 1024,
  });
  if (allowNoMatches && result.status === 1) return "";
  if (result.status !== 0) {
    throw new Error(result.stderr || `${arguments.join(" ")} failed`);
  }
  return result.stdout.trim();
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface Fixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: Record<string, string> & {
    readonly longhorn_selected_source_commit: string;
    readonly loophole_commit: string;
    readonly poodle_commit: string;
    readonly signal_commit: string;
    readonly soundcheck_commit: string;
  };
  readonly composition: Record<string, string | boolean | string[]> & {
    readonly hierarchy: string[];
    readonly canonical_app_id: string;
    readonly package_manager_publication: boolean;
    readonly fork_tree_packages: boolean;
    readonly excluded_later_systems: boolean;
  };
  readonly capability: {
    readonly identifier: string;
    readonly windows: string[];
  };
  readonly recovery: Record<string, boolean> & {
    readonly dual_write_authority: boolean;
    readonly silent_fallback: boolean;
    readonly automatic_source_cleanup: boolean;
    readonly cleanup_requires_exact_receipt_and_operator_authority: boolean;
  };
  readonly retained_adapters: string[];
  readonly deferred: string[];
}
