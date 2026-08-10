import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import {
  SURFACE_MUTATION_COMMAND_KINDS,
  SURFACE_MUTATION_OUTCOME_KINDS,
  SURFACE_MUTATION_REJECTION_CODES,
  SurfaceProtocolValidationError,
  assertValidSurfaceMutationCommand,
  assertValidSurfaceMutationOutcome,
  assertValidSurfaceMutationRejectionCode,
  assertValidSurfaceMutationResponse,
  assertValidSurfaceSnapshot,
  assertSurfaceProtocolVersion,
} from "@inflatable-cookie/longhorn/surfaces";

const fixturePath = new URL(
  "../../../../fixtures/surfaces/protocol-v1.json",
  import.meta.url,
);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

describe("Rust Surface protocol fixture", () => {
  test("round-trips every generated category without changing JSON", () => {
    assertSurfaceProtocolVersion(fixture.protocol_version);
    for (const category of [
      "snapshots",
      "commands",
      "receipts",
      "errors",
      "responses",
      "events",
    ]) {
      expect(roundTrip(fixture[category])).toEqual(fixture[category]);
    }
  });

  test("covers every mutation discriminant and rejection code", () => {
    const commands = array(fixture.commands).map((request) => {
      const command = record(record(request).command);
      assertValidSurfaceMutationCommand(command);
      return command.kind;
    });
    const outcomes = array(fixture.receipts).map((receipt) => {
      const outcome = record(record(receipt).outcome);
      assertValidSurfaceMutationOutcome(outcome);
      return outcome.kind;
    });
    const codes = array(fixture.errors).map((error) => {
      const code = record(error).code;
      assertValidSurfaceMutationRejectionCode(code);
      return code;
    });

    expect(new Set(commands)).toEqual(
      new Set(SURFACE_MUTATION_COMMAND_KINDS),
    );
    expect(new Set(outcomes)).toEqual(
      new Set(SURFACE_MUTATION_OUTCOME_KINDS),
    );
    expect(new Set(codes)).toEqual(
      new Set(SURFACE_MUTATION_REJECTION_CODES),
    );
    for (const response of array(fixture.responses)) {
      assertValidSurfaceMutationResponse(response);
    }
    for (const snapshot of array(fixture.snapshots)) {
      assertValidSurfaceSnapshot(snapshot);
    }
  });
});

describe("Surface protocol incompatibility", () => {
  const incompatibility = record(fixture.incompatibility);

  test("rejects future versions and unknown variants", () => {
    for (const check of [
      () =>
        assertSurfaceProtocolVersion(
          incompatibility.future_protocol_version,
        ),
      () =>
        assertValidSurfaceMutationCommand(
          incompatibility.unknown_command,
        ),
      () =>
        assertValidSurfaceMutationOutcome(
          incompatibility.unknown_outcome,
        ),
      () =>
        assertValidSurfaceMutationRejectionCode(
          incompatibility.unknown_rejection_code,
        ),
      () =>
        assertValidSurfaceMutationResponse({
          status: "future_surface_response",
        }),
    ]) {
      expect(check).toThrow(SurfaceProtocolValidationError);
    }
  });
});

function roundTrip(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value));
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("expected JSON object");
  }
  return value as Record<string, unknown>;
}

function array(value: unknown): unknown[] {
  if (!Array.isArray(value)) {
    throw new TypeError("expected JSON array");
  }
  return value;
}
