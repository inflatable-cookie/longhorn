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
      // oxlint-disable-next-line quality/no-known-value-widening -- Runtime validation also covers structurally typed callers and untrusted wire values.
      assertValidSurfaceMutationOutcome(record(receipt.outcome));
      return;
    }
    case "rejected": {
      record(response, variantKeys("SurfaceMutationResponse", response));
      const rejection = record(response.rejection, SURFACE_FIELDS.SurfaceMutationRejection);
      assertValidSurfaceMutationRejectionCode(rejection.code);
      return;
    }
    default:
      throw new SurfaceProtocolValidationError("unknown_response_status", response.status);
  }
}

export function assertValidSurfaceSnapshot(value: unknown): asserts value is SurfaceSnapshot {
  const snapshot = record(value, SURFACE_FIELDS.SurfaceSnapshot);
  assertSurfaceCursor(snapshot);
  const document = record(snapshot.document, SURFACE_FIELDS.SurfaceDocument);
  if (document.revision !== snapshot.revision) {
    throw new SurfaceProtocolValidationError("invalid_object", document);
  }
  for (const entry of array(document.surfaces)) {
    validateSurface(entry);
  }
  for (const entry of array(document.panel_instances)) {
    const panel = record(entry, SURFACE_FIELDS.PanelInstance);
    if (typeof panel.id !== "string" || typeof panel.definition_id !== "string") {
      throw new SurfaceProtocolValidationError("invalid_object", panel);
    }
  }
  for (const entry of array(document.windows)) {
    const window = record(entry, SURFACE_FIELDS.ParticipatingWindow);
    if (typeof window.id !== "string" || !nullableString(window.active_surface_id)) {
      throw new SurfaceProtocolValidationError("invalid_object", window);
    }
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
  if (
    typeof surface.id !== "string" ||
    typeof surface.schema_id !== "string" ||
    !nullableString(surface.label)
  ) {
    throw new SurfaceProtocolValidationError("invalid_object", surface);
  }
  const presentation = record(surface.presentation);
  record(presentation, variantKeys("SurfacePresentation", presentation));
  if (
    presentation.kind === "focused_panel" &&
    typeof presentation.panel_definition_id !== "string"
  ) {
    throw new SurfaceProtocolValidationError("invalid_object", presentation);
  }
  for (const entry of array(surface.regions)) {
    const region = record(entry, SURFACE_FIELDS.RegionState);
    if (
      typeof region.region_id !== "string" ||
      !nullableString(region.active_panel_instance_id) ||
      (region.collapsed !== null && typeof region.collapsed !== "boolean")
    ) {
      throw new SurfaceProtocolValidationError("invalid_object", region);
    }
    for (const id of array(region.panel_instance_ids)) {
      if (typeof id !== "string") {
        throw new SurfaceProtocolValidationError("invalid_object", region);
      }
    }
  }
  for (const entry of array(surface.sizing_slots)) {
    const slot = record(entry, SURFACE_FIELDS.SizingSlotState);
    if (typeof slot.sizing_slot_id !== "string" || !isLayoutRatio(slot.ratio)) {
      throw new SurfaceProtocolValidationError("invalid_object", slot);
    }
  }
  for (const entry of array(surface.host_preferences)) {
    const preference = record(entry, SURFACE_FIELDS.SurfaceHostPreference);
    if (typeof preference.window_id !== "string" || !unsigned32(preference.order)) {
      throw new SurfaceProtocolValidationError("invalid_object", preference);
    }
  }
}

function nullableString(value: unknown): boolean {
  return value === null || typeof value === "string";
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
  record(candidate, variantKeys(type, candidate));
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
