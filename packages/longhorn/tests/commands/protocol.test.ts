import { describe, expect, test } from "bun:test";

import {
  CommandProtocolValidationError,
  assertValidCommandAvailabilitySnapshot,
  assertValidCommandCatalogueChangedEvent,
  assertValidCommandCatalogue,
  assertValidCommandKeymapCommit,
  assertValidCommandKeymapChangedEvent,
  assertValidCommandKeymapLoadOutcome,
  assertValidCommandKeymapMutationResult,
  assertValidCommandKeymapPreview,
  assertValidCommandKeymapPreviewResult,
  assertValidCommandKeymapReset,
  assertValidCommandKeymapSnapshot,
} from "../../src/commands/index.ts";
import { fixture } from "./support.ts";
import { availability } from "./support.ts";

describe("generated command/keymap protocol fixture", () => {
  test("accepts every Rust-produced payload and discriminant", () => {
    assertValidCommandCatalogue(fixture.catalogue);
    assertValidCommandAvailabilitySnapshot(availability());
    assertValidCommandKeymapPreview(fixture.requests.preview);
    assertValidCommandKeymapCommit(fixture.requests.commit);
    assertValidCommandKeymapReset(fixture.requests.reset);
    assertValidCommandCatalogueChangedEvent(
      fixture.events.catalogueChanged,
    );
    assertValidCommandKeymapChangedEvent(fixture.events.keymapChanged);
    fixture.snapshots.forEach(assertValidCommandKeymapSnapshot);
    fixture.previewResults.forEach(
      assertValidCommandKeymapPreviewResult,
    );
    fixture.loadOutcomes.forEach(assertValidCommandKeymapLoadOutcome);
    fixture.mutationResults.forEach(
      assertValidCommandKeymapMutationResult,
    );
    expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
  });

  test("fails closed on future protocol and discriminants", () => {
    const catalogue = structuredClone(fixture.catalogue) as Record<
      string,
      unknown
    >;
    catalogue.protocolVersion = fixture.incompatibility.futureProtocolVersion;
    incompatible(() => assertValidCommandCatalogue(catalogue));

    for (const [collection, assertion, future] of [
      [
        fixture.previewResults,
        assertValidCommandKeymapPreviewResult,
        fixture.incompatibility.unknownPreviewStatus,
      ],
      [
        fixture.loadOutcomes,
        assertValidCommandKeymapLoadOutcome,
        fixture.incompatibility.unknownLoadStatus,
      ],
      [
        fixture.mutationResults,
        assertValidCommandKeymapMutationResult,
        fixture.incompatibility.unknownMutationStatus,
      ],
    ] as const) {
      const value = structuredClone(collection[0]) as Record<string, unknown>;
      value.status = future;
      incompatible(() => assertion(value));
    }

    const preview = structuredClone(fixture.requests.preview) as {
      patch: { upsertOverrides: Array<Record<string, unknown>> };
    };
    preview.patch.upsertOverrides[0]!.kind =
      fixture.incompatibility.unknownOverrideKind;
    incompatible(() => assertValidCommandKeymapPreview(preview));

    const rejected = structuredClone(
      fixture.previewResults.find(
        (result) => (result as { status?: unknown }).status === "rejected",
      ),
    ) as { rejection: { code: unknown } };
    rejected.rejection.code = fixture.incompatibility.unknownRejectionCode;
    incompatible(() => assertValidCommandKeymapPreviewResult(rejected));

    const recovery = structuredClone(
      fixture.loadOutcomes.find(
        (result) => (result as { status?: unknown }).status === "recovery",
      ),
    ) as { recovery: { code: unknown } };
    recovery.recovery.code = fixture.incompatibility.unknownRecoveryCode;
    incompatible(() => assertValidCommandKeymapLoadOutcome(recovery));

    const applied = structuredClone(
      fixture.mutationResults.find(
        (result) => (result as { status?: unknown }).status === "applied",
      ),
    ) as { receipt: { durability: unknown; outcome: unknown } };
    applied.receipt.durability = fixture.incompatibility.unknownDurability;
    incompatible(() => assertValidCommandKeymapMutationResult(applied));
    applied.receipt.durability = "fileSynced";
    applied.receipt.outcome = fixture.incompatibility.unknownMutationOutcome;
    incompatible(() => assertValidCommandKeymapMutationResult(applied));

    const snapshot = structuredClone(fixture.snapshots[0]) as {
      origin: { kind: unknown };
      bindings: Array<{ source: { kind: unknown } }>;
    };
    snapshot.origin.kind = fixture.incompatibility.unknownLoadOrigin;
    incompatible(() => assertValidCommandKeymapSnapshot(snapshot));
    snapshot.origin.kind = "default";
    snapshot.bindings[0]!.source.kind =
      fixture.incompatibility.unknownBindingSource;
    incompatible(() => assertValidCommandKeymapSnapshot(snapshot));

    const futureAvailability = availability() as {
      records: Array<{
        availability: {
          state: string;
          reason: unknown;
        };
      }>;
    };
    futureAvailability.records[0]!.availability = {
      state: "unavailable",
      reason: {
        code: { kind: "futureAvailability" },
        detail: null,
      },
    };
    incompatible(() =>
      assertValidCommandAvailabilitySnapshot(futureAvailability),
    );
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(CommandProtocolValidationError);
}
