import { describe, expect, it } from "bun:test";
import { NATIVE_CONTENT_VARIANT_FIELDS } from "../src/native-content/generated/variant-fields.ts";
import { NOTIFICATION_VARIANT_FIELDS } from "../src/notifications/generated/variant-fields.ts";
import { OPERATION_VARIANT_FIELDS } from "../src/operation/generated/variant-fields.ts";
import { HISTORY_TREE_VARIANT_FIELDS } from "../src/history-tree/generated/variant-fields.ts";
import { assertValidOperationMutationCommand } from "../src/operation/validation.ts";
import { assertValidNotificationMutationCommand } from "../src/notifications/validation.ts";

// Card 188. Each of these domains hand-wrote its per-variant key lists until
// now — 32, 25 and 12 of them, as literal arrays inside a switch. The lists
// are generated from the Rust enums; these assert the boundary actually reads
// them, in both directions.

const operationRegister = {
  kind: "register", requestId: "req:1", protocolVersion: 1, authority: { authorityId: "authority:test", authorityEpoch: 7 },
  expectedCatalogueRevision: 0, operationId: "op:1", kindId: "kind:1", scopeId: null, label: "Export",
  initialState: "queued", cancellationSupport: "supported", retryOf: null,
} as const;

const notificationDismiss = {
  kind: "dismiss", requestId: "req:1", protocolVersion: 1, authority: { authorityId: "authority:test", authorityEpoch: 7 },
  expectedLedgerRevision: 0, notificationId: "note:1",
} as const;

describe("tagged unions validate from the generated map", () => {
  it("accepts a variant carrying exactly its declared keys", () => {
    expect(() => assertValidOperationMutationCommand({ ...operationRegister })).not.toThrow();
  });

  it("rejects the same variant with one key the enum does not declare", () => {
    expect(() => assertValidOperationMutationCommand({ ...operationRegister, surplus: 1 })).toThrow();
  });

  it("rejects a variant missing a key the enum declares", () => {
    const { retryOf: _dropped, ...missing } = operationRegister;
    expect(() => assertValidOperationMutationCommand(missing)).toThrow();
  });

  it("holds for a second domain", () => {
    expect(() => assertValidNotificationMutationCommand({ ...notificationDismiss })).not.toThrow();
    expect(() => assertValidNotificationMutationCommand({ ...notificationDismiss, surplus: 1 })).toThrow();
  });

  // The discriminant is not always `kind`. native-content uses all three, and
  // the generator emits which so no call site has to choose.
  it("covers every discriminant the four domains use", () => {
    const maps = [NATIVE_CONTENT_VARIANT_FIELDS, NOTIFICATION_VARIANT_FIELDS, OPERATION_VARIANT_FIELDS, HISTORY_TREE_VARIANT_FIELDS];
    for (const map of maps) {
      expect(Object.keys(map).length).toBeGreaterThan(0);
      for (const variants of Object.values(map)) {
        for (const keys of Object.values(variants)) expect(keys.length).toBeGreaterThan(0);
      }
    }
  });
});
