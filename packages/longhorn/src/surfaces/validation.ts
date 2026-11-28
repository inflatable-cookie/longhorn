import { isOpaqueId } from "../core/identifiers.ts";
import { isLayoutRatio } from "../layout/ratio.ts";
import {
  checkWireFields,
  readWireRecord,
  unsigned32,
  unsignedInteger,
  wireVariantFields,
} from "../core/record.ts";
import {
  SURFACE_VARIANT_FIELDS,
  SURFACE_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import {
  SURFACE_MUTATION_COMMAND_KINDS,
  SURFACE_MUTATION_OUTCOME_KINDS,
  SURFACE_MUTATION_REJECTION_CODES,
  SURFACE_PROTOCOL_VERSION,
} from "./generated/protocol.ts";
import type {
  SurfaceChangedEvent,
  SurfaceDocument,
  SurfaceMutationCommand,
  SurfaceMutationOutcome,
  SurfaceMutationRejectionCode,
  SurfaceMutationResponse,
  SurfaceSnapshot,
} from "./generated/protocol.ts";
import { SURFACE_FIELDS } from "./generated/fields.ts";

export type SurfaceProtocolValidationCode =
  | "unsupported_protocol_version"
  | "unknown_command"
  | "unknown_outcome"
  | "unknown_rejection_code"
  | "unknown_response_status"
  | "uncorrelated_response"
  | "invalid_object"
  | "unknown_field"
  | "missing_field";

export class SurfaceProtocolValidationError extends Error {
  readonly code: SurfaceProtocolValidationCode;
  readonly received: unknown;

  constructor(code: SurfaceProtocolValidationCode, received: unknown) {
    super(`incompatible Surface protocol: ${code}`);
    this.name = "SurfaceProtocolValidationError";
    this.code = code;
    this.received = received;
  }
}

export function assertSurfaceProtocolVersion(
  version: unknown,
): asserts version is typeof SURFACE_PROTOCOL_VERSION {
  if (version !== SURFACE_PROTOCOL_VERSION) {
    throw new SurfaceProtocolValidationError("unsupported_protocol_version", version);
  }
}

export function assertValidSurfaceMutationCommand(
  value: unknown,
): asserts value is SurfaceMutationCommand {
  assertKnownKind(
    value,
    SURFACE_MUTATION_COMMAND_KINDS,
    "unknown_command",
    "SurfaceMutationCommand",
  );
}

export function assertValidSurfaceMutationOutcome(
  value: unknown,
): asserts value is SurfaceMutationOutcome {
  assertKnownKind(
    value,
    SURFACE_MUTATION_OUTCOME_KINDS,
    "unknown_outcome",
    "SurfaceMutationOutcome",
  );
}

export function assertValidSurfaceMutationRejectionCode(
  value: unknown,
): asserts value is SurfaceMutationRejectionCode {
  if (
    typeof value !== "string" ||
    !SURFACE_MUTATION_REJECTION_CODES.some((code) => code === value)
  ) {
    throw new SurfaceProtocolValidationError("unknown_rejection_code", value);
  }
}

export function assertValidSurfaceMutationResponse(
  value: unknown,
): asserts value is SurfaceMutationResponse {
  const response = record(value);
  switch (response.status) {
    case "committed": {
      record(response, variantKeys("SurfaceMutationResponse", response));
      const receipt = record(response.receipt, SURFACE_FIELDS.SurfaceMutationReceipt);
      assertFields(receipt, {
        request_id: isOpaqueId,
        previous_revision: unsignedInteger,
        committed_revision: unsignedInteger,
      });
      assertValidSurfaceMutationOutcome(receipt.outcome);
      assertValidSurfaceDocument(receipt.authoritative_document);
      return;
    }
    case "rejected": {
      record(response, variantKeys("SurfaceMutationResponse", response));
      const rejection = record(response.rejection, SURFACE_FIELDS.SurfaceMutationRejection);
      assertFields(rejection, {
        request_id: isOpaqueId,
        current_revision: unsignedInteger,
        detail: isString,
      });
      assertValidSurfaceMutationRejectionCode(rejection.code);
      assertValidSurfaceDocument(rejection.authoritative_document);
      return;
    }
    default:
      throw new SurfaceProtocolValidationError("unknown_response_status", response.status);
  }
}

export function assertValidSurfaceSnapshot(value: unknown): asserts value is SurfaceSnapshot {
  const snapshot = record(value, SURFACE_FIELDS.SurfaceSnapshot);
  assertSurfaceCursor(snapshot);
  const document = snapshot.document;
  assertValidSurfaceDocument(document);
  if (document.revision !== snapshot.revision) {
    throw new SurfaceProtocolValidationError("invalid_object", document);
  }
}

/** The whole authoritative document a snapshot, receipt or transfer carries. */
export function assertValidSurfaceDocument(value: unknown): asserts value is SurfaceDocument {
  const document = record(value, SURFACE_FIELDS.SurfaceDocument);
  assertFields(document, { revision: unsignedInteger });
  for (const entry of array(document.surfaces)) {
    validateSurface(entry);
  }
  for (const entry of array(document.panel_instances)) {
    assertFields(record(entry, SURFACE_FIELDS.PanelInstance), {
      id: isOpaqueId,
      definition_id: isOpaqueId,
    });
  }
  for (const entry of array(document.windows)) {
    assertFields(record(entry, SURFACE_FIELDS.ParticipatingWindow), {
      id: isOpaqueId,
      active_surface_id: isNullableId,
    });
  }
}

export function assertValidSurfaceChangedEvent(
  value: unknown,
): asserts value is SurfaceChangedEvent {
  assertSurfaceCursor(record(value, SURFACE_FIELDS.SurfaceChangedEvent));
}

function assertSurfaceCursor(value: Record<string, unknown>): void {
  assertSurfaceProtocolVersion(value.protocol_version);
  if (!unsignedInteger(value.epoch) || !unsignedInteger(value.revision)) {
    throw new SurfaceProtocolValidationError("invalid_object", value);
  }
}

function validateSurface(value: unknown): void {
  const surface = record(value, SURFACE_FIELDS.SurfaceRecord);
  assertFields(surface, { id: isOpaqueId, schema_id: isOpaqueId, label: isNullableString });
  validatePresentation(surface.presentation);
  for (const entry of array(surface.regions)) {
    const region = record(entry, SURFACE_FIELDS.RegionState);
    assertFields(region, {
      region_id: isOpaqueId,
      active_panel_instance_id: isNullableId,
      collapsed: isNullableBoolean,
    });
    assertIds(region.panel_instance_ids);
  }
  for (const entry of array(surface.sizing_slots)) {
    assertFields(record(entry, SURFACE_FIELDS.SizingSlotState), {
      sizing_slot_id: isOpaqueId,
      ratio: isLayoutRatio,
    });
  }
  validateHostPreferences(surface.host_preferences);
}

function validatePresentation(value: unknown): void {
  const presentation = record(value);
  record(presentation, variantKeys("SurfacePresentation", presentation));
  if (presentation.kind === "focused_panel") {
    assertFields(presentation, { panel_definition_id: isOpaqueId });
  }
}

function validateHostPreferences(value: unknown): void {
  for (const entry of array(value)) {
    assertFields(record(entry, SURFACE_FIELDS.SurfaceHostPreference), {
      window_id: isOpaqueId,
      order: unsigned32,
    });
  }
}

type MutationFields<Variant> = Variant extends { kind: string }
  ? Exclude<keyof Variant, "kind">
  : never;

// Each check throws, and a new Rust variant field fails to typecheck until it
// has one.
const mutationFields = {
  surface_id: field(isOpaqueId),
  source_surface_id: field(isOpaqueId),
  schema_id: field(isOpaqueId),
  window_id: field(isOpaqueId),
  source_window_id: field(isOpaqueId),
  target_window_id: field(isOpaqueId),
  previous_active_surface_id: field(isNullableId),
  label: field(isNullableString),
  insertion_index: field(unsigned32),
  surface_ids: assertIds,
  host_preferences: validateHostPreferences,
  presentation: validatePresentation,
  previous_presentation: validatePresentation,
} satisfies Record<
  MutationFields<SurfaceMutationCommand | SurfaceMutationOutcome>,
  (value: unknown) => void
>;

function field(admits: (value: unknown) => boolean): (value: unknown) => void {
  return (value) => {
    if (!admits(value)) {
      throw new SurfaceProtocolValidationError("invalid_object", value);
    }
  };
}

function assertFields(
  value: Record<string, unknown>,
  checks: Readonly<Record<string, (member: unknown) => boolean>>,
): void {
  for (const [key, admits] of Object.entries(checks)) {
    if (!admits(value[key])) {
      throw new SurfaceProtocolValidationError("invalid_object", { key, value });
    }
  }
}

function assertIds(value: unknown): void {
  for (const id of array(value)) {
    if (!isOpaqueId(id)) {
      throw new SurfaceProtocolValidationError("invalid_object", value);
    }
  }
}

function isString(value: unknown): boolean {
  return typeof value === "string";
}

function isNullableString(value: unknown): boolean {
  return value === null || isString(value);
}

function isNullableId(value: unknown): boolean {
  return value === null || isOpaqueId(value);
}

function isNullableBoolean(value: unknown): boolean {
  return value === null || typeof value === "boolean";
}

function array(value: unknown): ReadonlyArray<unknown> {
  if (!Array.isArray(value)) {
    throw new SurfaceProtocolValidationError("invalid_object", value);
  }
  return value;
}

function assertKnownKind(
  value: unknown,
  known: ReadonlyArray<string>,
  code: "unknown_command" | "unknown_outcome",
  type: string,
): asserts value is Record<"kind", string> {
  const candidate = record(value);
  if (typeof candidate.kind !== "string" || !known.includes(candidate.kind)) {
    throw new SurfaceProtocolValidationError(code, value);
  }
  const keys = variantKeys(type, candidate);
  record(candidate, keys);
  for (const [name, check] of Object.entries(mutationFields)) {
    if (keys.includes(name)) {
      check(candidate[name]);
    }
  }
}

function variantKeys(type: string, value: Record<string, unknown>): ReadonlyArray<string> {
  return wireVariantFields(
    SURFACE_VARIANT_FIELDS,
    SURFACE_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    (discriminant) => {
      throw new SurfaceProtocolValidationError("unknown_outcome", { type, discriminant });
    },
  );
}

function record(value: unknown, allowed?: ReadonlyArray<string>): Record<string, unknown> {
  const result = readWireRecord(value, () => {
    throw new SurfaceProtocolValidationError("invalid_object", value);
  });
  checkWireFields(result, allowed, (code, key) => {
    throw new SurfaceProtocolValidationError(code, { key, value });
  });
  return result;
}
