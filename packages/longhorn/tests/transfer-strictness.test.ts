import { describe, expect, test } from "bun:test";

import {
  assertValidTransferClientSnapshot,
  TransferProtocolValidationError,
} from "../src/transfer/validation.ts";
import { TRANSFER_FIELDS } from "../src/transfer/generated/fields.ts";
import { SURFACE_TRANSFER_FIELDS } from "../src/surface-transfer/generated/fields.ts";

/**
 * Contract 010's Boundary Validation Target on the transfer packages.
 *
 * Both previously checked "is this an object" and stopped, so a renamed or
 * dropped field passed straight through.
 */
describe("transfer boundary strictness", () => {
  const valid = {
    protocol_version: 1,
    client_id: "client:one",
    client_epoch: 1,
    current_lease_generation: 1,
  } as const;

  test("the field map matches the declared snapshot fields", () => {
    expect([...TRANSFER_FIELDS.TransferClientSnapshot!].sort()).toEqual(
      Object.keys(valid).sort(),
    );
  });

  test("rejects an unknown field", () => {
    expect(() =>
      assertValidTransferClientSnapshot({ ...valid, extra: 1 }),
    ).toThrow(TransferProtocolValidationError);
  });

  test("rejects a missing field", () => {
    const { client_id: _dropped, ...missing } = valid;
    expect(() => assertValidTransferClientSnapshot(missing)).toThrow(
      TransferProtocolValidationError,
    );
  });
});

describe("surface-transfer field map", () => {
  test("omits tagged unions rather than guessing their keys", () => {
    // Their allowed keys depend on a discriminant, so one flat list is wrong.
    // Handing `SurfaceTransferTarget` another type's list rejected `kind`.
    expect(SURFACE_TRANSFER_FIELDS.SurfaceTransferTarget).toBeUndefined();
    expect(SURFACE_TRANSFER_FIELDS.SurfaceTransferCompletion).toBeDefined();
  });
});
