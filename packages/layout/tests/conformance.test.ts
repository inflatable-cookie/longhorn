import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import {
  assertCompatibleLayoutMutationCommand,
  assertCompatibleLayoutMutationOutcome,
  assertCompatibleLayoutMutationRejectionCode,
  assertLayoutProtocolVersion,
} from "@longhorn/layout";

const fixtures = [
  load("loophole-conformance-v1.json"),
  load("nucleus-conformance-v1.json"),
];

const sharedCommandKinds = [
  "create_panel",
  "create_panel",
  "activate_panel",
  "reorder_region",
  "move_panel",
  "set_sizing_slot",
  "set_region_collapsed",
  "close_panel",
];

describe("two-shape layout conformance", () => {
  test("preserves each donor-shaped schema and external host binding", () => {
    const loophole = fixtures[0];
    const nucleus = fixtures[1];

    expect(loophole.name).toBe("loophole");
    expect(record(loophole.host_binding)).toMatchObject({
      kind: "surface",
      surface_id: "surface:mix",
      container_id: "container:primary",
    });
    expect(schemaRegions(loophole)).toHaveLength(8);
    expect(schemaSizingSlots(loophole)).toHaveLength(3);
    expect(loophole.resolved_default_region).toBe("primary");

    expect(nucleus.name).toBe("nucleus");
    expect(record(nucleus.host_binding)).toMatchObject({
      kind: "window",
      window_id: "window:project",
      container_id: "container:primary",
    });
    expect(schemaRegions(nucleus)).toHaveLength(5);
    expect(schemaSizingSlots(nucleus)).toHaveLength(4);
    expect(nucleus.resolved_default_region).toBe("main");
  });

  test("keeps host and product authority outside layout protocol state", () => {
    for (const fixture of fixtures) {
      const protocolState = JSON.stringify({
        definitions: fixture.definitions,
        initial_document: fixture.initial_document,
        steps: fixture.steps,
        expected_snapshot: fixture.expected_snapshot,
      });
      for (const forbidden of [
        "surface_id",
        "window_id",
        "title",
        "icon",
        "body",
        "resource",
        "runtime",
      ]) {
        expect(protocolState.includes(`"${forbidden}"`)).toBeFalse();
      }
    }
  });

  test("runs one complete command matrix through both shapes", () => {
    for (const fixture of fixtures) {
      assertLayoutProtocolVersion(fixture.protocol_version);
      const commandKinds = array(fixture.steps).map((step) => {
        const entry = record(step);
        const request = record(entry.request);
        const receipt = record(entry.receipt);
        const command = record(request.command);
        const outcome = record(receipt.outcome);
        assertCompatibleLayoutMutationCommand(command);
        assertCompatibleLayoutMutationOutcome(outcome);
        expect(receipt.previous_revision).toBe(request.expected_revision);
        expect(receipt.committed_revision).toBe(
          number(receipt.previous_revision) + 1,
        );
        return command.kind;
      });

      expect(commandKinds).toEqual(sharedCommandKinds);
      expect(record(fixture.expected_snapshot).revision).toBe(8);
      expect(fixture.expected_snapshot).toEqual(
        record(record(array(fixture.steps).at(-1)).receipt)
          .authoritative_document,
      );
    }
  });

  test("proves multiple and singleton policy without product payload", () => {
    for (const fixture of fixtures) {
      const snapshot = record(fixture.expected_snapshot);
      expect(array(snapshot.panel_instances)).toHaveLength(1);
      const singleton = record(fixture.singleton_policy);
      const firstReceipt = record(singleton.first_receipt);
      const rejection = record(singleton.second_rejection);
      assertCompatibleLayoutMutationRejectionCode(rejection.code);
      expect(rejection.code).toBe("instance_policy_exceeded");
      expect(rejection.authoritative_document).toEqual(
        firstReceipt.authoritative_document,
      );
    }
  });

  test("covers sizing, collapse, ordinary visibility, and transient reveal", () => {
    for (const fixture of fixtures) {
      const snapshot = record(fixture.expected_snapshot);
      const container = record(array(snapshot.containers)[0]);
      const sizing = record(array(container.sizing_slots)[0]);
      expect(sizing.ratio).toBe(300_000);
      expect(Number.isInteger(sizing.ratio)).toBeTrue();
      expect(
        array(container.regions)
          .map(record)
          .some((region) => region.collapsed === true),
      ).toBeTrue();

      const ordinaryStates = array(fixture.ordinary_visibility)
        .map(record)
        .map((visibility) => visibility.state);
      const transientStates = array(fixture.transient_visibility)
        .map(record)
        .map((visibility) => visibility.state);
      expect(ordinaryStates).toContain("visible");
      expect(ordinaryStates).toContain("hidden");
      expect(transientStates).toContain("transiently_revealed");
    }
  });

  test("preserves exact state for stale and invalid requests", () => {
    for (const fixture of fixtures) {
      const stale = record(fixture.stale_rejection);
      const invalid = record(fixture.invalid_rejection);
      assertCompatibleLayoutMutationRejectionCode(stale.code);
      assertCompatibleLayoutMutationRejectionCode(invalid.code);
      expect(stale.code).toBe("stale_revision");
      expect(invalid.code).toBe("move_target_unchanged");
      expect(stale.authoritative_document).toEqual(fixture.expected_snapshot);
      expect(invalid.authoritative_document).toEqual(
        fixture.expected_snapshot,
      );
    }
  });

  test("round-trips Rust expected snapshots exactly in TypeScript", () => {
    for (const fixture of fixtures) {
      expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
      expect(
        JSON.parse(JSON.stringify(fixture.expected_snapshot)),
      ).toEqual(fixture.expected_snapshot);
    }
  });
});

function load(name: string): Record<string, unknown> {
  const path = new URL(`../../../fixtures/layout/${name}`, import.meta.url);
  return record(JSON.parse(readFileSync(path, "utf8")));
}

function schemaRegions(fixture: Record<string, unknown>): unknown[] {
  return array(record(record(fixture.definitions).schema).regions);
}

function schemaSizingSlots(fixture: Record<string, unknown>): unknown[] {
  return array(record(record(fixture.definitions).schema).sizing_slots);
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

function number(value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeError("expected JSON number");
  }
  return value;
}
