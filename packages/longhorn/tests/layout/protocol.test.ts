import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import {
  LAYOUT_MUTATION_COMMAND_KINDS,
  LAYOUT_MUTATION_OUTCOME_KINDS,
  LAYOUT_MUTATION_REJECTION_CODES,
  LayoutProtocolValidationError,
  assertValidLayoutMutationCommand,
  assertValidLayoutMutationOutcome,
  assertValidLayoutMutationRejectionCode,
  assertLayoutProtocolVersion,
  layoutRatioFromMillionths,
  layoutRatioToUnitInterval,
  projectOrdinaryRegionVisibility,
} from "@inflatable-cookie/longhorn/layout";

const fixturePath = new URL("../../../../fixtures/layout/protocol-v1.json", import.meta.url);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

describe("Rust layout protocol fixture", () => {
  test("round-trips every generated category without changing JSON", () => {
    assertLayoutProtocolVersion(fixture.protocol_version);

    for (const category of [
      "definitions",
      "snapshots",
      "commands",
      "receipts",
      "errors",
      "visibility",
    ]) {
      expect(roundTrip(fixture[category])).toEqual(fixture[category]);
    }
  });

  test("covers every command, outcome, and rejection discriminant", () => {
    const commandKinds = array(fixture.commands).map((request) => {
      const command = record(record(request).command);
      assertValidLayoutMutationCommand(command);
      return command.kind;
    });
    const outcomeKinds = array(fixture.receipts).map((receipt) => {
      const outcome = record(record(receipt).outcome);
      assertValidLayoutMutationOutcome(outcome);
      return outcome.kind;
    });
    const rejectionCodes = array(fixture.errors).map((error) => {
      const code = record(error).code;
      assertValidLayoutMutationRejectionCode(code);
      return code;
    });

    expect(new Set(commandKinds)).toEqual(new Set(LAYOUT_MUTATION_COMMAND_KINDS));
    expect(new Set(outcomeKinds)).toEqual(new Set(LAYOUT_MUTATION_OUTCOME_KINDS));
    expect(new Set(rejectionCodes)).toEqual(new Set(LAYOUT_MUTATION_REJECTION_CODES));
  });

  test("preserves integer fixed-point ratios", () => {
    const definitions = record(fixture.definitions);
    const schema = record(array(definitions.schemas)[0]);
    const ratios = array(schema.sizing_slots).flatMap((slot) => {
      const sizingSlot = record(slot);
      return [sizingSlot.minimum, sizingSlot.default, sizingSlot.maximum];
    });

    for (const ratio of ratios) {
      expect(Number.isInteger(ratio)).toBeTrue();
      expect(layoutRatioFromMillionths(number(ratio))).toBe(number(ratio));
    }
    expect(layoutRatioToUnitInterval(layoutRatioFromMillionths(250_000))).toBe(0.25);
  });

  test("rejects non-integer and out-of-range ratios", () => {
    for (const invalid of [-1, 0.5, 1_000_001, Number.NaN]) {
      expect(() => layoutRatioFromMillionths(invalid)).toThrow(RangeError);
    }
    expect(layoutRatioFromMillionths(0)).toBe(0);
    expect(layoutRatioFromMillionths(1_000_000)).toBe(1_000_000);
  });

  test("matches Rust ordinary visibility projection", () => {
    const definitions = record(fixture.definitions);
    const schema = record(array(definitions.schemas)[0]);
    const snapshot = record(array(fixture.snapshots)[0]);
    const surface = record(array(snapshot.surfaces)[0]);
    const regions = array(schema.regions).map(record);
    const states = array(surface.regions).map(record);

    for (const definition of regions) {
      const state = states.find((candidate) => candidate.region_id === definition.id);
      expect(state).toBeDefined();
      const projected = projectOrdinaryRegionVisibility(definition as never, state as never);
      const rustProjection = array(fixture.visibility)
        .map(record)
        .find(
          (candidate) =>
            candidate.region_id === definition.id && candidate.state !== "transiently_revealed",
        );
      expect(rustProjection).toBeDefined();
      expect<unknown>(projected).toEqual(rustProjection);
    }
  });
});

describe("protocol incompatibility", () => {
  const incompatibility = record(fixture.incompatibility);

  test("rejects future protocol versions", () => {
    expect(() => assertLayoutProtocolVersion(incompatibility.future_protocol_version)).toThrow(
      LayoutProtocolValidationError,
    );
  });

  test("checks the fields of known commands and outcomes", () => {
    for (const command of [
      { kind: "create_panel" },
      { kind: "close_panel", panel_instance_id: "panel:one", extra: true },
      { kind: "close_panel", panel_instance_id: false },
      {
        kind: "reorder_region",
        surface_id: "surface:one",
        region_id: "left",
        panel_instance_ids: [1],
      },
      {
        kind: "move_panel",
        panel_instance_id: "panel:one",
        target_surface_id: "surface:one",
        target_region_id: "left",
        insertion_index: 2 ** 32,
      },
      {
        kind: "set_sizing_slot",
        surface_id: "surface:one",
        sizing_slot_id: "width",
        ratio: 1_000_001,
      },
    ]) {
      expect(() => assertValidLayoutMutationCommand(command)).toThrow(
        LayoutProtocolValidationError,
      );
    }
    for (const outcome of [
      { kind: "panel_closed" },
      {
        kind: "region_collapsed_set",
        surface_id: "surface:one",
        region_id: "left",
        previous_collapsed: null,
        committed_collapsed: true,
      },
      {
        kind: "panel_activated",
        surface_id: "surface:one",
        region_id: "left",
        panel_instance_id: "panel:one",
        previous_active_panel_instance_id: 1,
      },
    ]) {
      expect(() => assertValidLayoutMutationOutcome(outcome)).toThrow(
        LayoutProtocolValidationError,
      );
    }
  });

  test("rejects unknown future variants", () => {
    expect(() => assertValidLayoutMutationCommand(incompatibility.unknown_command)).toThrow(
      LayoutProtocolValidationError,
    );
    expect(() => assertValidLayoutMutationOutcome(incompatibility.unknown_outcome)).toThrow(
      LayoutProtocolValidationError,
    );
    expect(() =>
      assertValidLayoutMutationRejectionCode(incompatibility.unknown_rejection_code),
    ).toThrow(LayoutProtocolValidationError);
  });
});

function roundTrip(value: unknown): unknown {
  return structuredClone(value);
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("expected JSON object");
  }
  return value as Record<string, unknown>;
}

function array(value: unknown): Array<unknown> {
  if (!Array.isArray(value)) {
    throw new TypeError("expected JSON array");
  }
  return value;
}

function number(value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeError("expected JSON number");
  }
  return value;
}
