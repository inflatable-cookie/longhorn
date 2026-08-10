import { describe, expect, test } from "bun:test";

import {
  SettingsProtocolValidationError,
  assertValidSettingsApplyCommand,
  assertValidSettingsLoadCommand,
  assertValidSettingsLoadOutcome,
  assertValidSettingsMutationResult,
  assertValidSettingsRegistryChangedEvent,
  assertValidSettingsRegistrySnapshot,
  assertValidSettingsResetCommand,
  assertValidSettingsScopeChangedEvent,
  assertValidSettingsScopeSnapshot,
} from "../../src/settings/index.ts";
import { fixture } from "./support.ts";

describe("generated settings protocol fixture", () => {
  /**
   * The fixture publishes `recoveryStates`, and nothing exercised them: every
   * fixture snapshot carries `recovery: null`, so the recovery branch of
   * `assertValidSettingsScopeSnapshot` was never taken. Found by probing
   * the generated field lists — a sentinel key injected into
   * `SettingsRecoveryState` changed no test outcome, which meant that list was
   * enforcing nothing.
   */
  test("validates the recovery states the fixture publishes", () => {
    fixture.recoveryStates.forEach((state) => {
      const snapshot = structuredClone(fixture.snapshots[0]!) as Record<
        string,
        unknown
      >;
      snapshot.recovery = state;
      assertValidSettingsScopeSnapshot(snapshot);
    });
  });

  test("validates every Rust-produced payload category", () => {
    assertValidSettingsRegistrySnapshot(fixture.registry);
    fixture.snapshots.forEach((value) =>
      assertValidSettingsScopeSnapshot(value),
    );
    fixture.loadCommands.forEach((value) =>
      assertValidSettingsLoadCommand(value),
    );
    fixture.applyCommands.forEach((value) =>
      assertValidSettingsApplyCommand(value),
    );
    fixture.resetCommands.forEach((value) =>
      assertValidSettingsResetCommand(value),
    );
    fixture.loadOutcomes.forEach((value) =>
      assertValidSettingsLoadOutcome(value),
    );
    fixture.mutationResults.forEach((value) =>
      assertValidSettingsMutationResult(value),
    );
    fixture.registryEvents.forEach((value) =>
      assertValidSettingsRegistryChangedEvent(value),
    );
    fixture.scopeEvents.forEach((value) =>
      assertValidSettingsScopeChangedEvent(value),
    );
    expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
  });

  test("fails safe on future versions and every fixture discriminant", () => {
    const futureRegistry = structuredClone(fixture.registry);
    futureRegistry.protocolVersion =
      fixture.incompatibility.futureProtocolVersion;
    incompatible(() =>
      assertValidSettingsRegistrySnapshot(futureRegistry),
    );
    incompatible(() =>
      assertValidSettingsMutationResult(
        fixture.incompatibility.unknownMutationStatus,
      ),
    );
    incompatible(() =>
      assertValidSettingsLoadOutcome(
        fixture.incompatibility.unknownLoadStatus,
      ),
    );

    const durability = structuredClone(
      fixture.mutationResults[0]!,
    ) as { receipt: { durability: unknown } };
    durability.receipt.durability =
      fixture.incompatibility.unknownDurabilityKind;
    incompatible(() => assertValidSettingsMutationResult(durability));

    const rejection = structuredClone(
      fixture.mutationResults.find(
        (result) => result.status === "rejected",
      )!,
    ) as { rejection: { code: unknown } };
    rejection.rejection.code =
      fixture.incompatibility.unknownRejectionCode;
    incompatible(() => assertValidSettingsMutationResult(rejection));

    const recovery = structuredClone(
      fixture.snapshots[0]!,
    ) as unknown as { recovery: unknown };
    recovery.recovery = {
      code: fixture.incompatibility.unknownRecoveryCode,
      diagnostic: null,
    };
    incompatible(() => assertValidSettingsScopeSnapshot(recovery));

    const editability = structuredClone(fixture.snapshots[0]!);
    editability.values[0]!.editability =
      fixture.incompatibility.unknownEditability;
    incompatible(() => assertValidSettingsScopeSnapshot(editability));

    const timing = structuredClone(fixture.registry);
    timing.applyUnits[0]!.timing = "deferred";
    incompatible(() => assertValidSettingsRegistrySnapshot(timing));

    const effectiveSource = structuredClone(fixture.snapshots[0]!);
    effectiveSource.values[0]!.effectiveSource = "remote";
    incompatible(() =>
      assertValidSettingsScopeSnapshot(effectiveSource),
    );

    const policyEffect = structuredClone(fixture.snapshots[0]!);
    policyEffect.values[1]!.policy!.effect = "advisory";
    incompatible(() =>
      assertValidSettingsScopeSnapshot(policyEffect),
    );

    const activation = structuredClone(fixture.snapshots[0]!);
    activation.activationRequirements[0]!.state = "scheduled";
    incompatible(() => assertValidSettingsScopeSnapshot(activation));

    const mutationOutcome = structuredClone(
      fixture.mutationResults[0]!,
    ) as { receipt: { outcome: unknown } };
    mutationOutcome.receipt.outcome = "merged";
    incompatible(() =>
      assertValidSettingsMutationResult(mutationOutcome),
    );
  });

  test("rejects invalid tokens and oversized opaque values", () => {
    const invalidToken = structuredClone(fixture.snapshots[0]!);
    invalidToken.authority.authorityToken = "UPPERCASE";
    incompatible(() =>
      assertValidSettingsScopeSnapshot(invalidToken),
    );

    const oversized = structuredClone(
      fixture.snapshots[0]!,
    ) as unknown as {
      values: [{ effective: { value: unknown } }];
    };
    oversized.values[0].effective.value = "x".repeat(200);
    expect(() =>
      assertValidSettingsScopeSnapshot(oversized, 64),
    ).toThrow(
      expect.objectContaining({
        code: "opaque_value_too_large",
      }),
    );
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(SettingsProtocolValidationError);
}
