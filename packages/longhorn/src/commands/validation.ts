import { isOpaqueId } from "../core/identifiers.ts";
import { checkWireFields, readWireRecord, unsigned32, wireVariantFields } from "../core/record.ts";
import {
  COMMAND_VARIANT_FIELDS,
  COMMAND_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import {
  COMMAND_ARGUMENT_SCHEMA_SHAPES,
  COMMAND_AVAILABILITY_REASON_KINDS,
  COMMAND_AVAILABILITY_STATES,
  COMMAND_BINDING_SOURCE_KINDS,
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
  COMMAND_MAXIMUM_DIAGNOSTIC_BYTES,
  COMMAND_PLATFORM_SCOPES,
  COMMAND_PLATFORMS,
  COMMAND_TEXT_INPUT_POLICIES,
} from "./generated/protocol.ts";
import type {
  CommandCatalogueSnapshot,
  CommandAvailabilitySnapshot,
  CommandCatalogueChangedEvent,
  CommandKeymapChangedEvent,
  CommandKeymapCommit,
  CommandKeymapLoadOutcome,
  CommandKeymapMutationResult,
  CommandKeymapPreview,
  CommandKeymapPreviewResult,
  CommandKeymapReset,
  CommandKeymapSnapshot,
} from "./generated/protocol.ts";
import { COMMANDS_FIELDS } from "./generated/fields.ts";
import { isPhysicalCode } from "./physical-code.ts";

export class CommandProtocolValidationError extends Error {
  readonly code = "command_protocol_incompatible";

  constructor(
    readonly path: string,
    detail: string,
  ) {
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
  list(record.commands, "$.commands", discoveryRecord);
  list(record.presets, "$.presets", presetRecord);
}

export function assertValidCommandAvailabilitySnapshot(
  value: unknown,
): asserts value is CommandAvailabilitySnapshot {
  const record = object(value, "$", COMMANDS_FIELDS.CommandAvailabilitySnapshot);
  finiteInteger(record.registryGeneration, "$.registryGeneration");
  finiteInteger(record.contextRevision, "$.contextRevision");
  let previous = "";
  list(record.records, "$.records", (entry, path) => {
    const item = object(entry, path, COMMANDS_FIELDS.CommandAvailabilityRecord);
    const id = commandId(item.commandId, `${path}.commandId`);
    if (previous !== "" && id <= previous) {
      fail(`${path}.commandId`, "expected unique command-id order");
    }
    previous = id;
    availability(item.availability, `${path}.availability`);
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
  keymapSnapshot(value, "$");
}

export function assertValidCommandKeymapPreview(
  value: unknown,
): asserts value is CommandKeymapPreview {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapPreview);
  baseRequest(record, "$");
  keymapPatch(record.patch, "$.patch");
}

export function assertValidCommandKeymapCommit(
  value: unknown,
): asserts value is CommandKeymapCommit {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapCommit);
  version(record.protocolVersion, "$.protocolVersion");
  identifier(record.requestId, "$.requestId");
  commitEvidence(record.evidence, "$.evidence");
  keymapPatch(record.patch, "$.patch");
}

export function assertValidCommandKeymapReset(value: unknown): asserts value is CommandKeymapReset {
  const record = object(value, "$", COMMANDS_FIELDS.CommandKeymapReset);
  identifier(record.requestId, "$.requestId");
  baseRequest(record, "$");
}

export function assertValidCommandKeymapPreviewResult(
  value: unknown,
): asserts value is CommandKeymapPreviewResult {
  const record = object(value, "$");
  member(record.status, COMMAND_KEYMAP_PREVIEW_STATUSES, "$.status");
  object(record, "$", variantKeys("CommandKeymapPreviewResult", record, "$"));
  keymapSnapshot(record.snapshot, "$.snapshot");
  if (record.status === "accepted") {
    commitEvidence(record.evidence, "$.evidence");
  } else if (record.status === "rejected") {
    rejection(record.rejection, "$.rejection");
    list(record.conflicts, "$.conflicts", conflict);
  }
}

export function assertValidCommandKeymapLoadOutcome(
  value: unknown,
): asserts value is CommandKeymapLoadOutcome {
  const record = object(value, "$");
  member(record.status, COMMAND_KEYMAP_LOAD_STATUSES, "$.status");
  object(record, "$", variantKeys("CommandKeymapLoadOutcome", record, "$"));
  if (record.status === "loaded") {
    keymapSnapshot(record.snapshot, "$.snapshot");
  } else if (record.status === "recovery") {
    recoveryState(record.recovery, "$.recovery");
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
  keymapSnapshot(record.snapshot, "$.snapshot");
  if (record.status === "applied") {
    mutationReceipt(record.receipt, "$.receipt");
  } else if (record.status === "rejected") {
    rejection(record.rejection, "$.rejection");
    list(record.conflicts, "$.conflicts", conflict);
  }
}

function discoveryRecord(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandDiscoveryRecord);
  commandId(record.id, `${path}.id`);
  text(record.label, `${path}.label`);
  nullable(record.description, `${path}.description`, string);
  list(record.categoryPath, `${path}.categoryPath`, identifier);
  list(record.keywords, `${path}.keywords`, keyword);
  nullable(record.icon, `${path}.icon`, string);
  flags(record.visibility, `${path}.visibility`, COMMANDS_FIELDS.CommandVisibility);
  member(record.textInputPolicy, COMMAND_TEXT_INPUT_POLICIES, `${path}.textInputPolicy`);
  argumentSchema(record.arguments, `${path}.arguments`);
}

function presetRecord(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapPresetRecord);
  identifier(record.id, `${path}.id`);
  schemaVersion(record.version, `${path}.version`);
}

function availability(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandAvailability);
  member(record.state, COMMAND_AVAILABILITY_STATES, `${path}.state`);
  if ((record.state === "available") !== (record.reason === null)) {
    fail(`${path}.reason`, "available requires no reason; every other state requires one");
  }
  if (record.reason === null) {
    return;
  }
  const reason = object(record.reason, `${path}.reason`, COMMANDS_FIELDS.CommandAvailabilityReason);
  nullable(reason.detail, `${path}.reason.detail`, diagnosticText);
  const codePath = `${path}.reason.code`;
  const code = object(reason.code, codePath);
  member(code.kind, COMMAND_AVAILABILITY_REASON_KINDS, `${codePath}.kind`);
  object(code, codePath, variantKeys("CommandAvailabilityReasonCode", code, codePath));
  if (code.kind === "consumer") {
    identifier(code.code, `${codePath}.code`);
  }
}

function keymapSnapshot(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapSnapshot);
  version(record.protocolVersion, `${path}.protocolVersion`);
  finiteInteger(record.registryGeneration, `${path}.registryGeneration`);
  digest(record.registryDigest, `${path}.registryDigest`);
  const state = object(record.state, `${path}.state`, COMMANDS_FIELDS.CommandKeymapState);
  finiteInteger(state.revision, `${path}.state.revision`);
  identifier(state.activePresetId, `${path}.state.activePresetId`);
  list(state.overrides, `${path}.state.overrides`, keymapOverride);
  schemaVersion(record.activePresetVersion, `${path}.activePresetVersion`);
  list(record.bindings, `${path}.bindings`, effectiveBinding);
  list(record.conflicts, `${path}.conflicts`, conflict);
  loadOrigin(record.origin, `${path}.origin`);
  list(record.diagnostics, `${path}.diagnostics`, keymapDiagnostic);
}

function loadOrigin(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.kind, COMMAND_KEYMAP_LOAD_ORIGINS, `${path}.kind`);
  object(record, path, variantKeys("CommandKeymapLoadOrigin", record, path));
  if (record.kind === "migrated") {
    schemaVersion(record.from, `${path}.from`);
    schemaVersion(record.to, `${path}.to`);
  }
}

function keymapDiagnostic(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapDiagnostic);
  string(record.code, `${path}.code`);
  string(record.detail, `${path}.detail`);
}

function baseRequest(record: Record<string, unknown>, path: string): void {
  version(record.protocolVersion, `${path}.protocolVersion`);
  finiteInteger(record.registryGeneration, `${path}.registryGeneration`);
  finiteInteger(record.keymapRevision, `${path}.keymapRevision`);
  identifier(record.activePresetId, `${path}.activePresetId`);
  schemaVersion(record.activePresetVersion, `${path}.activePresetVersion`);
}

function commitEvidence(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapCommitEvidence);
  finiteInteger(record.registryGeneration, `${path}.registryGeneration`);
  finiteInteger(record.keymapRevision, `${path}.keymapRevision`);
  identifier(record.activePresetId, `${path}.activePresetId`);
  schemaVersion(record.activePresetVersion, `${path}.activePresetVersion`);
  digest(record.patchDigest, `${path}.patchDigest`);
}

function mutationReceipt(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapMutationReceipt);
  identifier(record.requestId, `${path}.requestId`);
  finiteInteger(record.previousRevision, `${path}.previousRevision`);
  finiteInteger(record.committedRevision, `${path}.committedRevision`);
  member(record.outcome, COMMAND_KEYMAP_MUTATION_OUTCOMES, `${path}.outcome`);
  member(record.durability, COMMAND_KEYMAP_DURABILITIES, `${path}.durability`);
  nullable(record.patchDigest, `${path}.patchDigest`, digest);
}

function recoveryState(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapRecovery);
  member(record.code, COMMAND_KEYMAP_RECOVERY_CODES, `${path}.code`);
  string(record.detail, `${path}.detail`);
  boolean(record.sourcePreserved, `${path}.sourcePreserved`);
}

function keymapPatch(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapPatch);
  nullable(record.activePresetId, `${path}.activePresetId`, identifier);
  boolean(record.clearOverrides, `${path}.clearOverrides`);
  list(record.removeBindingIds, `${path}.removeBindingIds`, identifier);
  list(record.upsertOverrides, `${path}.upsertOverrides`, keymapOverride);
}

function keymapOverride(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.kind, COMMAND_KEYMAP_OVERRIDE_KINDS, `${path}.kind`);
  object(record, path, variantKeys("CommandKeymapOverride", record, path));
  if (record.kind === "add") {
    const binding = object(
      record.binding,
      `${path}.binding`,
      COMMANDS_FIELDS.CommandBindingDefinition,
    );
    identifier(binding.id, `${path}.binding.id`);
    bindingPayload(binding, `${path}.binding`);
    return;
  }
  identifier(record.bindingId, `${path}.bindingId`);
  if (record.kind === "replace") {
    const replacement = object(
      record.replacement,
      `${path}.replacement`,
      COMMANDS_FIELDS.CommandBindingReplacement,
    );
    bindingPayload(replacement, `${path}.replacement`);
  }
}

/** The members a binding definition and its replacement share. */
function bindingPayload(record: Record<string, unknown>, path: string): void {
  member(record.platform, COMMAND_PLATFORM_SCOPES, `${path}.platform`);
  keyTrigger(record.trigger, `${path}.trigger`);
  identifier(record.contextId, `${path}.contextId`);
  commandId(record.commandId, `${path}.commandId`);
  // Raw JSON the registry validates; only a value JSON cannot carry is refused.
  if (record.arguments === undefined) {
    fail(`${path}.arguments`, "expected a JSON value");
  }
}

function effectiveBinding(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandEffectiveBinding);
  identifier(record.id, `${path}.id`);
  bindingSource(record.source, `${path}.source`);
  member(record.platform, COMMAND_PLATFORM_SCOPES, `${path}.platform`);
  keyTrigger(record.trigger, `${path}.trigger`);
  identifier(record.contextId, `${path}.contextId`);
  invocation(record.invocation, `${path}.invocation`);
}

function bindingSource(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.kind, COMMAND_BINDING_SOURCE_KINDS, `${path}.kind`);
  object(record, path, variantKeys("CommandBindingSource", record, path));
  if (record.kind !== "addedOverride") {
    identifier(record.presetId, `${path}.presetId`);
    schemaVersion(record.presetVersion, `${path}.presetVersion`);
  }
}

function keyTrigger(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeyTrigger);
  physicalCode(record.code, `${path}.code`);
  flags(record.modifiers, `${path}.modifiers`, COMMANDS_FIELDS.CommandTriggerModifiers);
}

function keyChord(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeyChord);
  physicalCode(record.code, `${path}.code`);
  flags(record.modifiers, `${path}.modifiers`, COMMANDS_FIELDS.CommandModifiers);
}

function invocation(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandInvocation);
  commandId(record.commandId, `${path}.commandId`);
  const values = object(record.arguments, `${path}.arguments`);
  for (const [field, argument] of Object.entries(values)) {
    identifier(field, `${path}.arguments.${field}`);
    argumentValue(argument, `${path}.arguments.${field}`);
  }
}

function conflict(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapConflict);
  member(record.platform, COMMAND_PLATFORMS, `${path}.platform`);
  keyChord(record.chord, `${path}.chord`);
  identifier(record.contextId, `${path}.contextId`);
  list(record.bindingIds, `${path}.bindingIds`, identifier);
  list(record.invocations, `${path}.invocations`, invocation);
}

function argumentSchema(value: unknown, path: string): void {
  const record = object(value, path);
  member(record.shape, COMMAND_ARGUMENT_SCHEMA_SHAPES, `${path}.shape`);
  object(record, path, variantKeys("CommandArgumentSchema", record, path));
  if (record.shape === "object") {
    list(record.fields, `${path}.fields`, argumentField);
  }
}

function argumentField(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandArgumentField);
  identifier(record.id, `${path}.id`);
  boolean(record.required, `${path}.required`);
  nullable(record.default, `${path}.default`, argumentValue);
  argumentKind(record.kind, `${path}.kind`);
}

// Both switches fail closed: a variant Rust adds is refused until it has a case.
function argumentKind(value: unknown, path: string): void {
  const record = object(value, path);
  object(record, path, variantKeys("CommandArgumentKind", record, path));
  switch (record.kind) {
    case "boolean":
      return;
    case "number":
      nullable(record.minimum, `${path}.minimum`, finiteNumber);
      nullable(record.maximum, `${path}.maximum`, finiteNumber);
      return;
    case "integer":
      nullable(record.minimum, `${path}.minimum`, wholeNumber);
      nullable(record.maximum, `${path}.maximum`, wholeNumber);
      return;
    case "string":
      finiteInteger(record.maximumBytes, `${path}.maximumBytes`);
      return;
    case "enum":
      list(record.values, `${path}.values`, identifier);
      return;
    default:
      fail(`${path}.kind`, "unknown argument kind");
  }
}

function argumentValue(value: unknown, path: string): void {
  const record = object(value, path);
  object(record, path, variantKeys("CommandArgumentValue", record, path));
  const valuePath = `${path}.value`;
  switch (record.kind) {
    case "boolean":
      boolean(record.value, valuePath);
      return;
    case "number":
      finiteNumber(record.value, valuePath);
      return;
    case "integer":
      // Rust holds an `i64`, so a whole number beyond the safe range is valid.
      wholeNumber(record.value, valuePath);
      return;
    case "string":
      string(record.value, valuePath);
      return;
    case "enum":
      identifier(record.value, valuePath);
      return;
    default:
      fail(`${path}.kind`, "unknown argument value kind");
  }
}

function rejection(value: unknown, path: string): void {
  const record = object(value, path, COMMANDS_FIELDS.CommandKeymapRejection);
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

function identifier(value: unknown, path: string): void {
  if (!isOpaqueId(value)) {
    fail(path, "expected bounded lowercase opaque id");
  }
}

// Command ids are the one identifier injected ports may supply outside the
// opaque grammar: contract 006 has the scorer fold them for search.
function commandId(value: unknown, path: string): string {
  text(value, path);
  return value;
}

function string(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string") {
    fail(path, "expected string");
  }
}

function text(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string" || value.length === 0) {
    fail(path, "expected non-empty string");
  }
}

function keyword(value: unknown, path: string): void {
  if (typeof value !== "string" || /^\p{White_Space}*$/u.test(value)) {
    fail(path, "expected non-blank keyword");
  }
}

function diagnosticText(value: unknown, path: string): void {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    new TextEncoder().encode(value).length > COMMAND_MAXIMUM_DIAGNOSTIC_BYTES
  ) {
    fail(path, "expected bounded non-empty diagnostic");
  }
}

function physicalCode(value: unknown, path: string): void {
  if (!isPhysicalCode(value)) {
    fail(path, "expected physical key code");
  }
}

function boolean(value: unknown, path: string): void {
  if (typeof value !== "boolean") {
    fail(path, "expected boolean");
  }
}

/** A record whose every generated member is a boolean. */
function flags(value: unknown, path: string, fields: ReadonlyArray<string>): void {
  const record = object(value, path, fields);
  for (const field of fields) {
    boolean(record[field], `${path}.${field}`);
  }
}

function schemaVersion(value: unknown, path: string): void {
  if (!unsigned32(value) || value === 0) {
    fail(path, "expected positive schema version");
  }
}

function finiteInteger(value: unknown, path: string): void {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    fail(path, "expected non-negative safe integer");
  }
}

function wholeNumber(value: unknown, path: string): void {
  if (!Number.isInteger(value)) {
    fail(path, "expected integer");
  }
}

function finiteNumber(value: unknown, path: string): void {
  if (!Number.isFinite(value)) {
    fail(path, "expected finite number");
  }
}

function nullable(
  value: unknown,
  path: string,
  check: (value: unknown, path: string) => void,
): void {
  if (value !== null) {
    check(value, path);
  }
}

function object(
  value: unknown,
  path: string,
  allowed?: ReadonlyArray<string>,
): Record<string, unknown> {
  const result = readWireRecord(value, () => fail(path, "expected object"));
  checkWireFields(result, allowed, (code, key) =>
    fail(`${path}.${key}`, code === "unknown_field" ? "unknown field" : "missing field"),
  );
  return result;
}

function list(value: unknown, path: string, check: (value: unknown, path: string) => void): void {
  if (!Array.isArray(value)) {
    fail(path, "expected array");
  }
  value.forEach((entry, index) => check(entry, `${path}[${index}]`));
}

function member<const T extends ReadonlyArray<string>>(
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

function variantKeys(
  type: string,
  value: Record<string, unknown>,
  path: string,
): ReadonlyArray<string> {
  return wireVariantFields(
    COMMAND_VARIANT_FIELDS,
    COMMAND_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    (discriminant) => fail(path, `no generated fields for ${type}.${String(discriminant)}`),
  );
}
