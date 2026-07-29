import {
  LAYOUT_MUTATION_COMMAND_KINDS,
  LAYOUT_MUTATION_OUTCOME_KINDS,
  LAYOUT_MUTATION_REJECTION_CODES,
  LAYOUT_PROTOCOL_VERSION,
  type LayoutMutationCommand,
  type LayoutMutationOutcome,
  type LayoutMutationRejectionCode,
} from "./generated/protocol.ts";

export type LayoutProtocolIncompatibilityCode =
  | "unsupported_protocol_version"
  | "unknown_command"
  | "unknown_outcome"
  | "unknown_rejection_code";

export class LayoutProtocolIncompatibilityError extends Error {
  readonly code: LayoutProtocolIncompatibilityCode;
  readonly received: unknown;

  constructor(code: LayoutProtocolIncompatibilityCode, received: unknown) {
    super(message(code, received));
    this.name = "LayoutProtocolIncompatibilityError";
    this.code = code;
    this.received = received;
  }
}

export function assertLayoutProtocolVersion(
  version: unknown,
): asserts version is typeof LAYOUT_PROTOCOL_VERSION {
  if (version !== LAYOUT_PROTOCOL_VERSION) {
    throw new LayoutProtocolIncompatibilityError(
      "unsupported_protocol_version",
      version,
    );
  }
}

export function assertCompatibleLayoutMutationCommand(
  value: unknown,
): asserts value is LayoutMutationCommand {
  assertKnownKind(value, LAYOUT_MUTATION_COMMAND_KINDS, "unknown_command");
}

export function assertCompatibleLayoutMutationOutcome(
  value: unknown,
): asserts value is LayoutMutationOutcome {
  assertKnownKind(value, LAYOUT_MUTATION_OUTCOME_KINDS, "unknown_outcome");
}

export function assertCompatibleLayoutMutationRejectionCode(
  value: unknown,
): asserts value is LayoutMutationRejectionCode {
  if (
    typeof value !== "string" ||
    !LAYOUT_MUTATION_REJECTION_CODES.includes(
      value as (typeof LAYOUT_MUTATION_REJECTION_CODES)[number],
    )
  ) {
    throw new LayoutProtocolIncompatibilityError(
      "unknown_rejection_code",
      value,
    );
  }
}

function assertKnownKind(
  value: unknown,
  known: readonly string[],
  code: "unknown_command" | "unknown_outcome",
): asserts value is Record<"kind", string> {
  if (
    typeof value !== "object" ||
    value === null ||
    !("kind" in value) ||
    typeof value.kind !== "string" ||
    !known.includes(value.kind)
  ) {
    throw new LayoutProtocolIncompatibilityError(code, value);
  }
}

function message(
  code: LayoutProtocolIncompatibilityCode,
  received: unknown,
): string {
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
