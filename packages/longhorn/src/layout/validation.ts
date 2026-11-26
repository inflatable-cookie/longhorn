import { checkWireFields, isRecord, unsigned32, wireVariantFields } from "../core/record.ts";
import {
  LAYOUT_VARIANT_FIELDS,
  LAYOUT_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import { isLayoutRatio } from "./ratio.ts";
import {
  LAYOUT_MUTATION_COMMAND_KINDS,
  LAYOUT_MUTATION_OUTCOME_KINDS,
  LAYOUT_MUTATION_REJECTION_CODES,
  LAYOUT_PROTOCOL_VERSION,
} from "./generated/protocol.ts";
import type {
  LayoutMutationCommand,
  LayoutMutationOutcome,
  LayoutMutationRejectionCode,
} from "./generated/protocol.ts";

export type LayoutProtocolValidationCode =
  | "unsupported_protocol_version"
  | "unknown_command"
  | "unknown_outcome"
  | "unknown_rejection_code";

export class LayoutProtocolValidationError extends Error {
  readonly code: LayoutProtocolValidationCode;
  readonly received: unknown;

  constructor(code: LayoutProtocolValidationCode, received: unknown, detail?: string) {
    super(detail ?? message(code, received));
    this.name = "LayoutProtocolValidationError";
    this.code = code;
    this.received = received;
  }
}

export function assertLayoutProtocolVersion(
  version: unknown,
): asserts version is typeof LAYOUT_PROTOCOL_VERSION {
  if (version !== LAYOUT_PROTOCOL_VERSION) {
    throw new LayoutProtocolValidationError("unsupported_protocol_version", version);
  }
}

export function assertValidLayoutMutationCommand(
  value: unknown,
): asserts value is LayoutMutationCommand {
  validateMutation(
    value,
    LAYOUT_MUTATION_COMMAND_KINDS,
    "LayoutMutationCommand",
    "unknown_command",
  );
}

export function assertValidLayoutMutationOutcome(
  value: unknown,
): asserts value is LayoutMutationOutcome {
  validateMutation(
    value,
    LAYOUT_MUTATION_OUTCOME_KINDS,
    "LayoutMutationOutcome",
    "unknown_outcome",
  );
}

export function assertValidLayoutMutationRejectionCode(
  value: unknown,
): asserts value is LayoutMutationRejectionCode {
  if (
    typeof value !== "string" ||
    !LAYOUT_MUTATION_REJECTION_CODES.some((code) => code === value)
  ) {
    throw new LayoutProtocolValidationError("unknown_rejection_code", value);
  }
}

type MutationFields<Variant> = Variant extends { kind: string }
  ? Exclude<keyof Variant, "kind">
  : never;

const mutationFields = {
  panel_instance_id: isString,
  panel_definition_id: isString,
  surface_id: isString,
  region_id: isString,
  target_surface_id: isString,
  target_region_id: isString,
  source_surface_id: isString,
  source_region_id: isString,
  sizing_slot_id: isString,
  insertion_index: unsigned32,
  former_index: unsigned32,
  panel_instance_ids: (value) => Array.isArray(value) && isStringList(value),
  ratio: isLayoutRatio,
  previous_ratio: isLayoutRatio,
  committed_ratio: isLayoutRatio,
  previous_active_panel_instance_id: (value) => value === null || isString(value),
  collapsed: isBoolean,
  previous_collapsed: isBoolean,
  committed_collapsed: isBoolean,
} satisfies Record<
  MutationFields<LayoutMutationCommand | LayoutMutationOutcome>,
  (value: unknown) => boolean
>;

function validateMutation(
  value: unknown,
  known: ReadonlyArray<string>,
  type: "LayoutMutationCommand" | "LayoutMutationOutcome",
  code: "unknown_command" | "unknown_outcome",
): void {
  if (!isRecord(value) || typeof value.kind !== "string" || !known.includes(value.kind)) {
    throw new LayoutProtocolValidationError(code, value);
  }
  const keys = wireVariantFields(
    LAYOUT_VARIANT_FIELDS,
    LAYOUT_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    () => {
      throw new LayoutProtocolValidationError(code, value);
    },
  );
  checkWireFields(value, keys, (issue, field) => {
    const detail = issue === "unknown_field" ? "unknown field" : "missing field";
    throw new LayoutProtocolValidationError(code, value, `${type}.${field}: ${detail}`);
  });
  for (const [field, validate] of Object.entries(mutationFields)) {
    if (keys.includes(field) && !validate(value[field])) {
      throw new LayoutProtocolValidationError(code, value, `${type}.${field}: invalid value`);
    }
  }
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

function isStringList(value: ReadonlyArray<unknown>): boolean {
  for (const item of value) {
    if (!isString(item)) {
      return false;
    }
  }
  return true;
}

function message(code: LayoutProtocolValidationCode, received: unknown): string {
  switch (code) {
    case "unsupported_protocol_version":
      return `unsupported layout protocol version ${String(received)}; expected ${LAYOUT_PROTOCOL_VERSION}`;
    case "unknown_command":
      return `unknown layout mutation command ${kindOf(received)}`;
    case "unknown_outcome":
      return `unknown layout mutation outcome ${kindOf(received)}`;
    case "unknown_rejection_code":
      return `unknown layout mutation rejection code ${String(received)}`;
  }
}

function kindOf(value: unknown): string {
  if (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    typeof value.kind === "string"
  ) {
    return value.kind;
  }
  return String(value);
}
