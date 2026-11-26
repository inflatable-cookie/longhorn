import { describe, expect, test } from "bun:test";

import {
  assertValidSurfaceChangedEvent,
  assertValidSurfaceSnapshot,
  SurfaceProtocolValidationError,
} from "../../src/surfaces/validation.ts";
import { SURFACE_PROTOCOL_VERSION } from "../../src/surfaces/generated/protocol.ts";

/**
 * Contract 010's Boundary Validation Target, on the first package migrated to
 * it. Surfaces previously checked "is this an object" and stopped, so a
 * renamed or dropped field passed straight through.
 */
const valid = {
  protocol_version: SURFACE_PROTOCOL_VERSION,
  epoch: 1,
  revision: 1,
  document: { revision: 1, surfaces: [], windows: [], panel_instances: [] },
};

describe("surfaces boundary strictness", () => {
  test("accepts exactly the declared fields", () => {
    expect(() => assertValidSurfaceSnapshot(valid)).not.toThrow();
  });

  test("rejects an unknown field", () => {
    expect(() => assertValidSurfaceSnapshot({ ...valid, extra: 1 })).toThrow(
      SurfaceProtocolValidationError,
    );
  });

  test("rejects a missing field", () => {
    const { epoch: _dropped, ...missing } = valid;
    expect(() => assertValidSurfaceSnapshot(missing)).toThrow(SurfaceProtocolValidationError);
  });

  test("rejects invalid ordering cursors on snapshots and events", () => {
    const { document: _document, ...event } = valid;
    for (const change of [
      { epoch: "1" },
      { revision: -1 },
      { revision: Number.MAX_SAFE_INTEGER + 1 },
    ]) {
      expect(() => assertValidSurfaceSnapshot({ ...valid, ...change })).toThrow(
        SurfaceProtocolValidationError,
      );
      expect(() => assertValidSurfaceChangedEvent({ ...event, ...change })).toThrow(
        SurfaceProtocolValidationError,
      );
    }
  });

  test("checks the document and its nested members", () => {
    for (const document of [
      null,
      { ...valid.document, surfaces: [null] },
      { ...valid.document, windows: [{ id: "window:main", active_surface_id: false }] },
      { ...valid.document, panel_instances: [{ id: "panel:one", definition_id: 3 }] },
      { ...valid.document, revision: 0.5 },
      { ...valid.document, revision: 2 },
    ]) {
      expect(() => assertValidSurfaceSnapshot({ ...valid, document })).toThrow(
        SurfaceProtocolValidationError,
      );
    }
  });

  test("reports a non-object as invalid_object", () => {
    // It used to report `unknown_response_status`, because the union had no
    // code for a non-object. Wrong for every caller but one.
    try {
      assertValidSurfaceSnapshot([]);
      throw new Error("expected a rejection");
    } catch (error) {
      expect((error as SurfaceProtocolValidationError).code).toBe("invalid_object");
    }
  });
});
