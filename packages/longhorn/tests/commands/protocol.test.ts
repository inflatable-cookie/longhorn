import { describe, expect, test } from "bun:test";

import {
  CommandProtocolIncompatibilityError,
  assertCompatibleCommandAvailabilitySnapshot,
  assertCompatibleCommandCatalogueChangedEvent,
  assertCompatibleCommandCatalogue,
  assertCompatibleCommandKeymapCommit,
  assertCompatibleCommandKeymapChangedEvent,
  assertCompatibleCommandKeymapLoadOutcome,
  assertCompatibleCommandKeymapMutationResult,
  assertCompatibleCommandKeymapPreview,
  assertCompatibleCommandKeymapPreviewResult,
  assertCompatibleCommandKeymapReset,
  assertCompatibleCommandKeymapSnapshot,
} from "../../src/commands/index.ts";
import { fixture } from "./support.ts";
import { availability } from "./support.ts";

describe("generated command/keymap protocol fixture", () => {
  test("accepts every Rust-produced payload and discriminant", () => {
    assertCompatibleCommandCatalogue(fixture.catalogue);
    assertCompatibleCommandAvailabilitySnapshot(availability());
    assertCompatibleCommandKeymapPreview(fixture.requests.preview);
    assertCompatibleCommandKeymapCommit(fixture.requests.commit);
    assertCompatibleCommandKeymapReset(fixture.requests.reset);
    assertCompatibleCommandCatalogueChangedEvent(
      fixture.events.catalogueChanged,
    );
    assertCompatibleCommandKeymapChangedEvent(fixture.events.keymapChanged);
    fixture.snapshots.forEach(assertCompatibleCommandKeymapSnapshot);
    fixture.previewResults.forEach(
      assertCompatibleCommandKeymapPreviewResult,
    );
    fixture.loadOutcomes.forEach(assertCompatibleCommandKeymapLoadOutcome);
    fixture.mutationResults.forEach(
      assertCompatibleCommandKeymapMutationResult,
    );
    expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
  });

  test("fails closed on future protocol and discriminants", () => {
    const catalogue = structuredClone(fixture.catalogue) as Record<
      string,
      unknown
    >;
    catalogue.protocolVersion = fixture.incompatibility.futureProtocolVersion;
    incompatible(() => assertCompatibleCommandCatalogue(catalogue));

    for (const [collection, assertion, future] of [
      [
        fixture.previewResults,
        assertCompatibleCommandKeymapPreviewResult,
        fixture.incompatibility.unknownPreviewStatus,
      ],
      [
        fixture.loadOutcomes,
        assertCompatibleCommandKeymapLoadOutcome,
        fixture.incompatibility.unknownLoadStatus,
      ],
      [
        fixture.mutationResults,
        assertCompatibleCommandKeymapMutationResult,
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
    incompatible(() => assertCompatibleCommandKeymapPreview(preview));

    const rejected = structuredClone(
      fixture.previewResults.find(
        (result) => (result as { status?: unknown }).status === "rejected",
      ),
    ) as { rejection: { code: unknown } };
    rejected.rejection.code = fixture.incompatibility.unknownRejectionCode;
    incompatible(() => assertCompatibleCommandKeymapPreviewResult(rejected));

    const recovery = structuredClone(
      fixture.loadOutcomes.find(
        (result) => (result as { status?: unknown }).status === "recovery",
      ),
    ) as { recovery: { code: unknown } };
    recovery.recovery.code = fixture.incompatibility.unknownRecoveryCode;
    incompatible(() => assertCompatibleCommandKeymapLoadOutcome(recovery));

    const applied = structuredClone(
      fixture.mutationResults.find(
        (result) => (result as { status?: unknown }).status === "applied",
      ),
    ) as { receipt: { durability: unknown; outcome: unknown } };
    applied.receipt.durability = fixture.incompatibility.unknownDurability;
    incompatible(() => assertCompatibleCommandKeymapMutationResult(applied));
    applied.receipt.durability = "fileSynced";
    applied.receipt.outcome = fixture.incompatibility.unknownMutationOutcome;
    incompatible(() => assertCompatibleCommandKeymapMutationResult(applied));

    const snapshot = structuredClone(fixture.snapshots[0]) as {
      origin: { kind: unknown };
      bindings: Array<{ source: { kind: unknown } }>;
    };
    snapshot.origin.kind = fixture.incompatibility.unknownLoadOrigin;
    incompatible(() => assertCompatibleCommandKeymapSnapshot(snapshot));
    snapshot.origin.kind = "default";
    snapshot.bindings[0]!.source.kind =
      fixture.incompatibility.unknownBindingSource;
    incompatible(() => assertCompatibleCommandKeymapSnapshot(snapshot));

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
      assertCompatibleCommandAvailabilitySnapshot(futureAvailability),
    );
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(CommandProtocolIncompatibilityError);
}
