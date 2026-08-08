import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

type SourceCheck = {
  path: string;
  contains: string[];
  excludes?: string[];
};

const longhornRoot = resolve(import.meta.dir, "..");
const loopholeRoot = resolve(
  process.env.LOOPHOLE_REPO ?? resolve(longhornRoot, "../loophole"),
);
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/loophole-card110/settings-command-cutover-v1.json",
    ),
    "utf8",
  ),
) as {
  schema: string;
  outcome: string;
  settings: {
    pages: string[];
    backup_restore_admission: { status: string };
  };
  commands: {
    generic_tauri_execute_by_string: boolean;
    contexts: string[];
  };
  legacy_keymap_migration: { excluded: string[] };
  retained_extended_input: { owner: string };
};

assertEqual(
  fixture.schema,
  "longhorn.loophole-settings-command-cutover.v1",
  "fixture schema",
);
assertEqual(
  fixture.outcome,
  "pass_with_retained_product_edges",
  "fixture outcome",
);
assertEqual(fixture.settings.pages.length, 6, "settings page count");
assertEqual(
  fixture.settings.backup_restore_admission.status,
  "not_admitted",
  "backup/restore admission",
);
assertEqual(
  fixture.commands.generic_tauri_execute_by_string,
  false,
  "generic execute-by-string",
);
assertEqual(fixture.commands.contexts.length, 15, "command context count");
assertEqual(
  fixture.legacy_keymap_migration.excluded.length,
  8,
  "retained trigger classes",
);

const checks: SourceCheck[] = [
  {
    path: "aura/src-tauri/src/services/settings_host.rs",
    contains: [
      'const APP_UNIT: &str = "loophole:app.apply"',
      'const APPEARANCE_UNIT: &str = "loophole:appearance.apply"',
      "SettingsMutationTiming::Immediate",
      "SettingsMutationTiming::Staged",
      "register_command_settings",
      "register_config_operations_settings",
      "STORAGE_DIAGNOSTICS_CAPABILITY_ID",
      'assert!(!pages.contains(&"longhorn:backup"))',
      'assert!(!pages.contains(&"longhorn:restore"))',
      "APPEARANCE_CHANGED_EVENT",
      "sync_autosave",
    ],
  },
  {
    path: "aura/src-tauri/src/services/command_host.rs",
    contains: [
      "CommandRegistryBuilder::new",
      "echo_command_surface::command_registry()",
      "legacy_override_directives",
      "legacy_last_override_wins_and_disables_shadowed_base_binding",
      "retained_macro_override_shadows_longhorn_base_without_becoming_generic",
      '"commands/keymap.json"',
      '"commands/legacy-echo-import-v1.json"',
    ],
  },
  {
    path: "aura/src/renderer/workspace/workspace-command-session.svelte.ts",
    contains: [
      "new CommandController",
      "new CommandSession",
      "loadAvailability",
      "availabilityEntry",
      "executeWorkspaceSharedCommand",
      '"longhorn_command_keymap_preview"',
      '"longhorn_command_keymap_commit"',
    ],
    excludes: ["invoke(commandId"],
  },
  {
    path: "aura/src/renderer/workspace/workspace-extended-input-adapter.ts",
    contains: [
      "LoopholeExtendedInputAdapter",
      'action.kind !== "macro"',
      "executeResolvedWorkspaceKeyboardAction",
      "workspaceCommandController.dispatchInvocation",
    ],
  },
  {
    path: "aura/src/renderer/settings/SettingsModal.svelte",
    contains: [
      "SettingsShell",
      'app: "loophole:app"',
      'appearance: "loophole:appearance"',
      'keybindings: "longhorn:keybindings"',
      'case "longhorn:config.storage"',
    ],
  },
  {
    path: "aura/src/renderer/settings/SettingsKeybindingsTab.svelte",
    contains: [
      "KeybindingSettings",
      "workspaceCommandSession.projection?.conflicts",
      "workspaceCommandSession.stagePatch",
    ],
  },
  {
    path: "aura/src/renderer/workspace/CommandPalette.svelte",
    contains: ["CommandPaletteBinding", "workspaceCommandSession"],
  },
  {
    path: "aura/src-tauri/src/commands/keyboard.rs",
    contains: ["get_tauri_extended_input_overrides"],
    excludes: ["set_tauri_keymap", "resolve_tauri_keybinding"],
  },
];

for (const check of checks) {
  const source = readFileSync(resolve(loopholeRoot, check.path), "utf8");
  for (const token of check.contains) {
    assert(source.includes(token), `${check.path} lost ${JSON.stringify(token)}`);
  }
  for (const token of check.excludes ?? []) {
    assert(!source.includes(token), `${check.path} gained ${JSON.stringify(token)}`);
  }
}

for (const removed of [
  "aura/src-tauri/src/adapters/keyboard.rs",
  "aura/src/renderer/host-provider/keyboard-host-provider.ts",
  "aura/src/renderer/settings/chord-capture.ts",
  "aura/src/renderer/settings/keybinding-conflicts.ts",
  "aura/src/renderer/settings/keymap-overrides-wire.ts",
]) {
  assert(!existsSync(resolve(loopholeRoot, removed)), `${removed} still exists`);
}

const binding = readFileSync(
  resolve(
    longhornRoot,
    "packages/longhorn-poodle-svelte/src/commands/poodle/CommandPaletteBinding.svelte",
  ),
  "utf8",
);
assert(binding.includes("onOpenChange?: (open: boolean) => void"), "palette close seam missing");

console.log(
  JSON.stringify(
    {
      schema: "longhorn.loophole-settings-command-verification.v1",
      outcome: fixture.outcome,
      settingsPages: fixture.settings.pages,
      sourceChecks: checks.length,
      retainedOwner: fixture.retained_extended_input.owner,
      removedGenericPaths: 5,
    },
    null,
    2,
  ),
);

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (actual !== expected) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}
