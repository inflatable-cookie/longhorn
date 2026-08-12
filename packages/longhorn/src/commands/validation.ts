import {
  COMMAND_VARIANT_FIELDS,
  COMMAND_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import {
  COMMAND_KEYMAP_DURABILITIES,
  COMMAND_KEYMAP_LOAD_ORIGINS,
  COMMAND_KEYMAP_LOAD_STATUSES,
  COMMAND_KEYMAP_MUTATION_OUTCOMES,
  COMMAND_KEYMAP_MUTATION_STATUSES,
  COMMAND_KEYMAP_OVERRIDE_KINDS,
  COMMAND_KEYMAP_PREVIEW_STATUSES,
  COMMAND_KEYMAP_PROTOCOL_VERSION,
  COMMAND_KEYMAP_RECOVERY_CODES,
  COMMAND_KEYMAP_REJECTION_CODES,
  type CommandCatalogueSnapshot,
  type CommandAvailabilitySnapshot,
  type CommandCatalogueChangedEvent,
  type CommandKeymapChangedEvent,
  type CommandKeymapCommit,
  type CommandKeymapLoadOutcome,
  type CommandKeymapMutationResult,
  type CommandKeymapPreview,
  type CommandKeymapPreviewResult,
  type CommandKeymapReset,
  type CommandKeymapSnapshot,
} from "./generated/protocol.ts";
import { COMMANDS_FIELDS } from "./generated/fields.ts";

export class CommandProtocolValidationError extends Error {
  readonly code = "command_protocol_incompatible";

  constructor(readonly path: string, detail: string) {
    super(`${path}: ${detail}`);
    this.name = "CommandProtocolValidationError";
  }
}

export function assertValidCommandCatalogue(
  value: unknown,
): asserts value is CommandCatalogueSnapshot {
  const record = object(value, "$", COMMANDS_FIELDS.CommandCatalogueSnapshot);
  version(record.protocolVersion, "$.protocolVersion");
  finiteInteger(record.registryGeneration, "$.registryGeneration");
  digest(record.registryDigest, "$.registryDigest");
  array(record.commands, "$.commands").forEach((command, index) => {
    const item = object(command, `$.commands[${index}]`);
    text(item.id, `$.commands[${index}].id`);
    text(item.label, `$.commands[${index}].label`);
    argumentSchema(item.arguments, `$.commands[${index}].arguments`);
  });
  array(record.presets, "$.presets").forEach((preset, index) => {
    const item = object(preset, `$.presets[${index}]`);
    text(item.id, `$.presets[${index}].id`);
    positiveInteger(item.version, `$.presets[${index}].version`);
  });
}

export function assertValidCommandAvailabilitySnapshot(
  value: unknown,
): asserts value is CommandAvailabilitySnapshot {
  const record = object(value, "$", COMMANDS_FIELDS.CommandAvailabilitySnapshot);
  finiteInteger(record.registryGeneration, "$.registryGeneration");
  finiteInteger(record.contextRevision, "$.contextRevision");
  let previous = "";
  array(record.records, "$.records").forEach((entry, index) => {
    const item = object(entry, `$.records[${index}]`);
    const commandId = text(item.commandId, `$.records[${index}].commandId`);
    if (index > 0 && commandId <= previous) {
      fail(
        `$.records[${index}].commandId`,
        "expected unique command-id order",
      );
    }
    previous = commandId;
    const availability = object(
      item.availability,
      `$.records[${index}].availability`,
    );
    member(
      availability.state,
      ["available", "unavailable", "hidden", "unsupported"],
      `$.records[${index}].availability.state`,
    );
    if (
      (availability.state === "available") !==
      (availability.reason === null)
    ) {
      fail(
        `$.records[${index}].availability.reason`,
        "available requires no reason; every other state requires one",
      );
    }
    if (availability.reason !== null) {
      const reason = object(
        availability.reason,
        `$.records[${index}].availability.reason`,
      );
      if (reason.detail !== null) {
        text(
          reason.detail,
          `$.records[${index}].availability.reason.detail`,
        );
      }
      const code = object(
        reason.code,
        `$.records[${index}].availability.reason.code`,
      );
      member(
        code.kind,
        ["contextNotAllowed", "missingCapability", "consumer"],
        `$.records[${index}].availability.reason.code.kind`,
      );
      if (code.kind === "consumer") {
        text(
          code.code,
          `$.records[${index}].availability.reason.code.code`,
        );
      }
    }
  });
}

export function assertValidCommandCatalogueChangedEvent(
  value: unknown,
): asserts value is CommandCatalogueChangedEvent {
  const record = object(value, "$", COMMANDS_FIELDS.CommandCatalogueChangedEvent);
  version(record.protocolVersion, "$.protocolVersion");
  finiteInteger(record.registryGeneration, "$.registryGeneration");
}

export function assertValidCommandKeymapChangedEvent(
  value: unknown,
): asserts value is CommandKeymapChangedEvent {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapChangedEvent);
  version(record.protocolVersion, "$.protocolVersion");
  finiteInteger(record.registryGeneration, "$.registryGeneration");
  finiteInteger(record.keymapRevision, "$.keymapRevision");
}

export function assertValidCommandKeymapSnapshot(
  value: unknown,
): asserts value is CommandKeymapSnapshot {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapSnapshot);
  version(record.protocolVersion, "$.protocolVersion");
  finiteInteger(record.registryGeneration, "$.registryGeneration");
  digest(record.registryDigest, "$.registryDigest");
  const state = object(record.state, "$.state");
  finiteInteger(state.revision, "$.state.revision");
  text(state.activePresetId, "$.state.activePresetId");
  array(state.overrides, "$.state.overrides").forEach((override, index) =>
    keymapOverride(override, `$.state.overrides[${index}]`),
  );
  positiveInteger(record.activePresetVersion, "$.activePresetVersion");
  array(record.bindings, "$.bindings").forEach((binding, index) =>
    effectiveBinding(binding, `$.bindings[${index}]`),
  );
  array(record.conflicts, "$.conflicts");
  const origin = object(record.origin, "$.origin");
  member(origin.kind, COMMAND_KEYMAP_LOAD_ORIGINS, "$.origin.kind");
  object(origin, "$.origin", variantKeys("CommandKeymapLoadOrigin", origin, "$.origin"));
  if (origin.kind === "migrated") {
    positiveInteger(origin.from, "$.origin.from");
    positiveInteger(origin.to, "$.origin.to");
  }
  array(record.diagnostics, "$.diagnostics");
}

export function assertValidCommandKeymapPreview(
  value: unknown,
): asserts value is CommandKeymapPreview {
  baseRequest(value, "$", COMMANDS_FIELDS.CommandKeymapPreview);
  keymapPatch(object(value, "$").patch, "$.patch");
}

export function assertValidCommandKeymapCommit(
  value: unknown,
): asserts value is CommandKeymapCommit {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapCommit);
  text(record.requestId, "$.requestId");
  const evidence = object(record.evidence, "$.evidence");
  finiteInteger(evidence.registryGeneration, "$.evidence.registryGeneration");
  finiteInteger(evidence.keymapRevision, "$.evidence.keymapRevision");
  text(evidence.activePresetId, "$.evidence.activePresetId");
  positiveInteger(
    evidence.activePresetVersion,
    "$.evidence.activePresetVersion",
  );
  digest(evidence.patchDigest, "$.evidence.patchDigest");
  keymapPatch(record.patch, "$.patch");
}

export function assertValidCommandKeymapReset(
  value: unknown,
): asserts value is CommandKeymapReset {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapReset);
  text(record.requestId, "$.requestId");
  baseRequest(value, "$");
}

export function assertValidCommandKeymapPreviewResult(
  value: unknown,
): asserts value is CommandKeymapPreviewResult {
  const record = object(value, "$");
  member(record.status, COMMAND_KEYMAP_PREVIEW_STATUSES, "$.status");
  object(record, "$", variantKeys("CommandKeymapPreviewResult", record, "$"));
  assertValidCommandKeymapSnapshot(record.snapshot);
  if (record.status === "accepted") {
    const evidence = object(record.evidence, "$.evidence");
    digest(evidence.patchDigest, "$.evidence.patchDigest");
  } else if (record.status === "rejected") {
    rejection(record.rejection, "$.rejection");
    array(record.conflicts, "$.conflicts");
  }
}

export function assertValidCommandKeymapLoadOutcome(
  value: unknown,
): asserts value is CommandKeymapLoadOutcome {
  const record = object(value, "$");
  member(record.status, COMMAND_KEYMAP_LOAD_STATUSES, "$.status");
  object(record, "$", variantKeys("CommandKeymapLoadOutcome", record, "$"));
  if (record.status === "loaded") {
    assertValidCommandKeymapSnapshot(record.snapshot);
  } else if (record.status === "recovery") {
    const recovery = object(record.recovery, "$.recovery");
    member(recovery.code, COMMAND_KEYMAP_RECOVERY_CODES, "$.recovery.code");
  } else {
    text(record.detail, "$.detail");
  }
}

export function assertValidCommandKeymapMutationResult(
  value: unknown,
): asserts value is CommandKeymapMutationResult {
  const record = object(value, "$");
  member(record.status, COMMAND_KEYMAP_MUTATION_STATUSES, "$.status");
  object(record, "$", variantKeys("CommandKeymapMutationResult", record, "$"));
  assertValidCommandKeymapSnapshot(record.snapshot);
  if (record.status === "applied") {
    const receipt = object(record.receipt, "$.receipt");
    member(
      receipt.outcome,
      COMMAND_KEYMAP_MUTATION_OUTCOMES,
      "$.receipt.outcome",
    );
    member(
      receipt.durability,
      COMMAND_KEYMAP_DURABILITIES,
      "$.receipt.durability",
    );
  } else if (record.status === "rejected") {
    rejection(record.rejection, "$.rejection");
    array(record.conflicts, "$.conflicts");
  }
}

function baseRequest(
  value: unknown,
  path: string,
  allowed?: readonly string[],
): void {
  const record = object(value, path, allowed);
  finiteInteger(record.registryGeneration, `${path}.registryGeneration`);
  finiteInteger(record.keymapRevision, `${path}.keymapRevision`);
  text(record.activePresetId, `${path}.activePresetId`);
  positiveInteger(record.activePresetVersion, `${path}.activePresetVersion`);
}

function keymapPatch(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapPatch);
  if (
    record.activePresetId !== null &&
    typeof record.activePresetId !== "string"
  ) {
    fail(`${path}.activePresetId`, "expected string or null");
  }
  if (typeof record.clearOverrides !== "boolean") {
    fail(`${path}.clearOverrides`, "expected boolean");
  }
  array(record.removeBindingIds, `${path}.removeBindingIds`).forEach(
    (id, index) => text(id, `${path}.removeBindingIds[${index}]`),
  );
  array(record.upsertOverrides, `${path}.upsertOverrides`).forEach(
    (override, index) =>
      keymapOverride(override, `${path}.upsertOverrides[${index}]`),
  );
}

function keymapOverride(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.kind, COMMAND_KEYMAP_OVERRIDE_KINDS, `${path}.kind`);
  object(record, path, variantKeys("CommandKeymapOverride", record, path));
  if (record.kind === "add") {
    bindingDefinition(record.binding, `${path}.binding`);
  } else {
    text(record.bindingId, `${path}.bindingId`);
    if (record.kind === "replace") {
      bindingDefinition(record.replacement, `${path}.replacement`, false);
    }
  }
}

function bindingDefinition(
  value: unknown,
  path: string,
  requireId = true,
): void {
  const record = object(value, path);
  if (requireId) text(record.id, `${path}.id`);
  text(record.commandId, `${path}.commandId`);
  text(record.contextId, `${path}.contextId`);
  const trigger = object(record.trigger, `${path}.trigger`);
  text(trigger.code, `${path}.trigger.code`);
}

function effectiveBinding(value: unknown, path: string): void {
  const record = object(value, path);
  text(record.id, `${path}.id`);
  const source = object(record.source, `${path}.source`);
  member(source.kind, ["preset", "replacement", "addedOverride"], `${path}.source.kind`);
  object(source, `${path}.source`, variantKeys("CommandBindingSource", source, `${path}.source`));
  bindingDefinition(
    {
      ...record,
      commandId: object(record.invocation, `${path}.invocation`).commandId,
    },
    path,
  );
}

function argumentSchema(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.shape, ["none", "object"], `${path}.shape`);
  object(record, path, variantKeys("CommandArgumentSchema", record, path));
  if (record.shape === "object") array(record.fields, `${path}.fields`);
}

function rejection(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.code, COMMAND_KEYMAP_REJECTION_CODES, `${path}.code`);
  text(record.detail, `${path}.detail`);
}

function version(value: unknown, path: string): void {
  if (value !== COMMAND_KEYMAP_PROTOCOL_VERSION) {
    fail(path, `expected protocol ${COMMAND_KEYMAP_PROTOCOL_VERSION}`);
  }
}

function digest(value: unknown, path: string): void {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
    fail(path, "expected lowercase SHA-256 digest");
  }
}

function text(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0) {
    fail(path, "expected non-empty string");
  }
  return value;
}

function positiveInteger(value: unknown, path: string): void {
  finiteInteger(value, path);
  if ((value as number) < 1) fail(path, "expected positive integer");
}

function finiteInteger(value: unknown, path: string): void {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < 0
  ) {
    fail(path, "expected non-negative safe integer");
  }
}

/**
 * Rejects a non-object, an unknown key, and a missing key.
 *
 * `allowed` comes from the generated field map, so the keys accepted are the
 * Rust struct's and nothing else — contract 010's Boundary Validation Target.
 * Passing no list keeps shape-only behaviour for the tagged unions, whose
 * allowed keys depend on their discriminant and so are not one flat set.
 */
function object(
  value: unknown,
  path: string,
  allowed?: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail(path, "expected object");
  }
  const record = value as Record<string, unknown>;
  if (allowed === undefined) return record;

  const permitted = new Set(allowed);
  for (const key of Object.keys(record)) {
    if (!permitted.has(key)) fail(`${path}.${key}`, "unknown field");
  }
  for (const key of allowed) {
    if (!(key in record)) fail(`${path}.${key}`, "missing field");
  }
  return record;
}

function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail(path, "expected array");
  return value;
}

function member<const T extends readonly string[]>(
  value: unknown,
  choices: T,
  path: string,
): asserts value is T[number] {
  if (typeof value !== "string" || !choices.includes(value)) {
    fail(path, `expected one of ${choices.join(", ")}`);
  }
}

function fail(path: string, detail: string): never {
  throw new CommandProtocolValidationError(path, detail);
}

/**
 * Allowed keys for one tagged-union variant, from the generated map, with the
 * discriminant's name read from the map too. This domain tags on `kind`,
 * `shape` and `status`.
 *
 * A missing entry means the generator failed to read the union — every caller
 * runs `member()` over the discriminant above this call.
 */
function variantKeys(
  type: string,
  value: Record<string, unknown>,
  path: string,
): readonly string[] {
  const discriminant = value[COMMAND_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const keys = COMMAND_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) fail(path, `no generated fields for ${type}.${String(discriminant)}`);
  return keys;
}
