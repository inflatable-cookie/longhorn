import { describe, expect, test } from "bun:test";

import {
  assertValidCommandAvailabilitySnapshot,
  assertValidCommandCatalogue,
  assertValidCommandKeymapCommit,
  assertValidCommandKeymapLoadOutcome,
  assertValidCommandKeymapMutationResult,
  assertValidCommandKeymapPreview,
  assertValidCommandKeymapPreviewResult,
  assertValidCommandKeymapReset,
  assertValidCommandKeymapSnapshot,
} from "../../src/commands/index.ts";
import type {
  CommandArgumentSchema,
  CommandArguments,
  CommandAvailabilitySnapshot,
  CommandCatalogueSnapshot,
  CommandEffectiveBinding,
  CommandKeymapCommit,
  CommandKeymapConflict,
  CommandKeymapMutationResult,
  CommandKeymapPatch,
  CommandKeymapPreview,
  CommandKeymapPreviewResult,
  CommandKeymapReset,
  CommandKeymapSnapshot,
  CommandKeyTrigger,
} from "../../src/commands/index.ts";
import { expectStrict } from "../corruption.ts";

const digest = "0".repeat(64);
// Bindings carry raw JSON arguments; invocations carry a map of typed ones.
const options = {
  raw: (path: string) => /\.(?:binding|replacement)\.arguments$/.test(path),
  map: (path: string) => /\.(?:invocation|invocations\.\d+)\.arguments$/.test(path),
};

const argumentsByKind: CommandArguments = {
  flag: { kind: "boolean", value: true },
  ratio: { kind: "number", value: 0.5 },
  count: { kind: "integer", value: 2 },
  name: { kind: "string", value: "" },
  mode: { kind: "enum", value: "fast" },
};

const schema: CommandArgumentSchema = {
  shape: "object",
  fields: [
    {
      id: "flag",
      required: true,
      default: { kind: "boolean", value: false },
      kind: { kind: "boolean" },
    },
    {
      id: "ratio",
      required: false,
      default: { kind: "number", value: 1.5 },
      kind: { kind: "number", minimum: 0, maximum: 2 },
    },
    {
      id: "count",
      required: false,
      default: { kind: "integer", value: 1 },
      kind: { kind: "integer", minimum: 0, maximum: 9 },
    },
    {
      id: "name",
      required: false,
      default: { kind: "string", value: "a" },
      kind: { kind: "string", maximumBytes: 8 },
    },
    {
      id: "mode",
      required: false,
      default: { kind: "enum", value: "fast" },
      kind: { kind: "enum", values: ["fast"] },
    },
  ],
};

const catalogue: CommandCatalogueSnapshot = {
  protocolVersion: 1,
  registryGeneration: 1,
  registryDigest: digest,
  commands: [
    {
      id: "app:open",
      label: "Open",
      description: "Opens",
      categoryPath: ["general"],
      keywords: ["file"],
      icon: "open",
      visibility: {
        hidden: false,
        palette: true,
        menu: true,
        shortcut: true,
        settings: true,
        help: true,
      },
      textInputPolicy: "blocked",
      arguments: schema,
    },
  ],
  presets: [{ id: "app:default", version: 1 }],
};

const availability: CommandAvailabilitySnapshot = {
  registryGeneration: 1,
  contextRevision: 1,
  records: [
    {
      commandId: "app:open",
      availability: {
        state: "unavailable",
        reason: { code: { kind: "consumer", code: "editor:busy" }, detail: "busy" },
      },
    },
    {
      commandId: "app:save",
      availability: {
        state: "hidden",
        reason: { code: { kind: "contextNotAllowed" }, detail: null },
      },
    },
  ],
};

const trigger: CommandKeyTrigger = {
  code: "KeyO",
  modifiers: { primary: true, control: false, alt: false, shift: false, meta: false },
};

const binding: CommandEffectiveBinding = {
  id: "base:open",
  source: { kind: "preset", presetId: "app:default", presetVersion: 1 },
  platform: "any",
  trigger,
  contextId: "global",
  invocation: { commandId: "app:open", arguments: argumentsByKind },
};

const conflict: CommandKeymapConflict = {
  platform: "macOs",
  chord: { code: "KeyO", modifiers: { control: false, alt: false, shift: false, meta: true } },
  contextId: "global",
  bindingIds: ["base:open"],
  invocations: [{ commandId: "app:open", arguments: argumentsByKind }],
};

const replacement = {
  platform: "linux",
  trigger,
  contextId: "global",
  commandId: "app:open",
  arguments: { anything: ["raw"] },
} as const;

const patch: CommandKeymapPatch = {
  activePresetId: "app:default",
  clearOverrides: false,
  removeBindingIds: ["base:save"],
  upsertOverrides: [
    { kind: "disable", bindingId: "base:open" },
    { kind: "replace", bindingId: "base:open", replacement },
    { kind: "add", binding: { id: "user:open", ...replacement } },
  ],
};

const snapshot: CommandKeymapSnapshot = {
  protocolVersion: 1,
  registryGeneration: 1,
  registryDigest: digest,
  state: { revision: 1, activePresetId: "app:default", overrides: patch.upsertOverrides },
  activePresetVersion: 1,
  bindings: [
    binding,
    {
      ...binding,
      id: "base:save",
      source: { kind: "replacement", presetId: "app:default", presetVersion: 1 },
    },
    { ...binding, id: "user:open", source: { kind: "addedOverride" } },
  ],
  conflicts: [conflict],
  origin: { kind: "migrated", from: 1, to: 2 },
  diagnostics: [{ code: "missing", detail: "no file" }],
};

const preview: CommandKeymapPreview = {
  protocolVersion: 1,
  registryGeneration: 1,
  keymapRevision: 1,
  activePresetId: "app:default",
  activePresetVersion: 1,
  patch,
};

const evidence = {
  registryGeneration: 1,
  keymapRevision: 1,
  activePresetId: "app:default",
  activePresetVersion: 1,
  patchDigest: digest,
};

const commit: CommandKeymapCommit = {
  protocolVersion: 1,
  requestId: "request:commit",
  evidence,
  patch,
};

const reset: CommandKeymapReset = {
  protocolVersion: 1,
  requestId: "request:reset",
  registryGeneration: 1,
  keymapRevision: 1,
  activePresetId: "app:default",
  activePresetVersion: 1,
};

const rejection = { code: "conflict", detail: "taken" } as const;

const previewResults: Array<CommandKeymapPreviewResult> = [
  { status: "accepted", evidence, snapshot },
  { status: "rejected", rejection, snapshot, conflicts: [conflict] },
];

const mutationResults: Array<CommandKeymapMutationResult> = [
  {
    status: "applied",
    snapshot,
    receipt: {
      requestId: "request:commit",
      previousRevision: 1,
      committedRevision: 2,
      outcome: "changed",
      durability: "fileSynced",
      patchDigest: digest,
    },
  },
  { status: "rejected", rejection, snapshot, conflicts: [conflict] },
];

describe("commands nested boundary strictness", () => {
  test("refuses any corrupted member of a catalogue or availability snapshot", () => {
    expectStrict(assertValidCommandCatalogue, catalogue, options);
    expectStrict(assertValidCommandAvailabilitySnapshot, availability, options);
  });

  test("refuses any corrupted member of a keymap snapshot or request", () => {
    expectStrict(assertValidCommandKeymapSnapshot, snapshot, options);
    expectStrict(assertValidCommandKeymapPreview, preview, options);
    expectStrict(assertValidCommandKeymapCommit, commit, options);
    expectStrict(assertValidCommandKeymapReset, reset, options);
  });

  test("refuses any corrupted member of a keymap reply", () => {
    for (const result of previewResults) {
      expectStrict(assertValidCommandKeymapPreviewResult, result, options);
    }
    for (const result of mutationResults) {
      expectStrict(assertValidCommandKeymapMutationResult, result, options);
    }
    expectStrict(
      assertValidCommandKeymapLoadOutcome,
      {
        status: "recovery",
        recovery: { code: "corrupt", detail: "bad json", sourcePreserved: true },
      },
      options,
    );
  });

  test("refuses a binding without modifiers before a keypress can read them", () => {
    const { modifiers: _dropped, ...bare } = trigger;
    expect(() =>
      assertValidCommandKeymapSnapshot({
        ...snapshot,
        bindings: [{ ...binding, trigger: bare }],
      }),
    ).toThrow("$.bindings[0].trigger.modifiers: missing field");
  });

  test("refuses a command without a text-input policy", () => {
    const [command] = catalogue.commands;
    const { textInputPolicy: _dropped, ...bare } = command!;
    expect(() => assertValidCommandCatalogue({ ...catalogue, commands: [bare] })).toThrow(
      "$.commands[0].textInputPolicy: missing field",
    );
  });

  test("admits only identifiers and keywords Rust would admit", () => {
    const [command] = catalogue.commands;
    const withCommand = (change: Partial<NonNullable<typeof command>>) => () =>
      assertValidCommandCatalogue({ ...catalogue, commands: [{ ...command, ...change }] });
    expect(withCommand({ categoryPath: ["_internal"] })).not.toThrow();
    expect(withCommand({ categoryPath: ["General"] })).toThrow("$.commands[0].categoryPath[0]");
    expect(withCommand({ keywords: ["  "] })).toThrow("$.commands[0].keywords[0]");
    // Contract 006: an injected port's command id is folded for search, not refused.
    expect(withCommand({ id: "app:InspectFile" })).not.toThrow();
  });

  test("admits an integer bound beyond the safe range, which Rust holds as i64", () => {
    const [command] = catalogue.commands;
    expect(() =>
      assertValidCommandCatalogue({
        ...catalogue,
        commands: [
          {
            ...command,
            arguments: {
              shape: "object",
              fields: [
                {
                  id: "count",
                  required: false,
                  default: null,
                  // oxlint-disable-next-line eslint/no-loss-of-precision -- The wire value of i64::MAX.
                  kind: { kind: "integer", minimum: null, maximum: 9_223_372_036_854_775_807 },
                },
              ],
            },
          },
        ],
      }),
    ).not.toThrow();
  });
});
