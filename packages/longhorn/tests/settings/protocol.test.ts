import { describe, expect, test } from "bun:test";

import {
  SettingsProtocolIncompatibilityError,
  assertCompatibleSettingsApplyCommand,
  assertCompatibleSettingsLoadCommand,
  assertCompatibleSettingsLoadOutcome,
  assertCompatibleSettingsMutationResult,
  assertCompatibleSettingsRegistryChangedEvent,
  assertCompatibleSettingsRegistrySnapshot,
  assertCompatibleSettingsResetCommand,
  assertCompatibleSettingsScopeChangedEvent,
  assertCompatibleSettingsScopeSnapshot,
} from "../../src/settings/index.ts";
import { fixture } from "./support.ts";

describe("generated settings protocol fixture", () => {
  test("validates every Rust-produced payload category", () => {
    assertCompatibleSettingsRegistrySnapshot(fixture.registry);
    fixture.snapshots.forEach((value) =>
      assertCompatibleSettingsScopeSnapshot(value),
    );
    fixture.loadCommands.forEach((value) =>
      assertCompatibleSettingsLoadCommand(value),
    );
    fixture.applyCommands.forEach((value) =>
      assertCompatibleSettingsApplyCommand(value),
    );
    fixture.resetCommands.forEach((value) =>
      assertCompatibleSettingsResetCommand(value),
    );
    fixture.loadOutcomes.forEach((value) =>
      assertCompatibleSettingsLoadOutcome(value),
    );
    fixture.mutationResults.forEach((value) =>
      assertCompatibleSettingsMutationResult(value),
    );
    fixture.registryEvents.forEach((value) =>
      assertCompatibleSettingsRegistryChangedEvent(value),
    );
    fixture.scopeEvents.forEach((value) =>
      assertCompatibleSettingsScopeChangedEvent(value),
    );
    expect(JSON.parse(JSON.stringify(fixture))).toEqual(fixture);
  });

  test("fails safe on future versions and every fixture discriminant", () => {
    const futureRegistry = structuredClone(fixture.registry);
    futureRegistry.protocolVersion =
      fixture.incompatibility.futureProtocolVersion;
    incompatible(() =>
      assertCompatibleSettingsRegistrySnapshot(futureRegistry),
    );
    incompatible(() =>
      assertCompatibleSettingsMutationResult(
        fixture.incompatibility.unknownMutationStatus,
      ),
    );
    incompatible(() =>
      assertCompatibleSettingsLoadOutcome(
        fixture.incompatibility.unknownLoadStatus,
      ),
    );

    const durability = structuredClone(
      fixture.mutationResults[0]!,
    ) as { receipt: { durability: unknown } };
    durability.receipt.durability =
      fixture.incompatibility.unknownDurabilityKind;
    incompatible(() => assertCompatibleSettingsMutationResult(durability));

    const rejection = structuredClone(
      fixture.mutationResults.find(
        (result) => result.status === "rejected",
      )!,
    ) as { rejection: { code: unknown } };
    rejection.rejection.code =
      fixture.incompatibility.unknownRejectionCode;
    incompatible(() => assertCompatibleSettingsMutationResult(rejection));

    const recovery = structuredClone(
      fixture.snapshots[0]!,
    ) as unknown as { recovery: unknown };
    recovery.recovery = {
      code: fixture.incompatibility.unknownRecoveryCode,
      diagnostic: null,
    };
    incompatible(() => assertCompatibleSettingsScopeSnapshot(recovery));

    const editability = structuredClone(fixture.snapshots[0]!);
    editability.values[0]!.editability =
      fixture.incompatibility.unknownEditability;
    incompatible(() => assertCompatibleSettingsScopeSnapshot(editability));

    const timing = structuredClone(fixture.registry);
    timing.applyUnits[0]!.timing = "deferred";
    incompatible(() => assertCompatibleSettingsRegistrySnapshot(timing));

    const effectiveSource = structuredClone(fixture.snapshots[0]!);
    effectiveSource.values[0]!.effectiveSource = "remote";
    incompatible(() =>
      assertCompatibleSettingsScopeSnapshot(effectiveSource),
    );

    const policyEffect = structuredClone(fixture.snapshots[0]!);
    policyEffect.values[1]!.policy!.effect = "advisory";
    incompatible(() =>
      assertCompatibleSettingsScopeSnapshot(policyEffect),
    );

    const activation = structuredClone(fixture.snapshots[0]!);
    activation.activationRequirements[0]!.state = "scheduled";
    incompatible(() => assertCompatibleSettingsScopeSnapshot(activation));

    const mutationOutcome = structuredClone(
      fixture.mutationResults[0]!,
    ) as { receipt: { outcome: unknown } };
    mutationOutcome.receipt.outcome = "merged";
    incompatible(() =>
      assertCompatibleSettingsMutationResult(mutationOutcome),
    );
  });

  test("rejects invalid tokens and oversized opaque values", () => {
    const invalidToken = structuredClone(fixture.snapshots[0]!);
    invalidToken.authority.authorityToken = "UPPERCASE";
    incompatible(() =>
      assertCompatibleSettingsScopeSnapshot(invalidToken),
    );

    const oversized = structuredClone(
      fixture.snapshots[0]!,
    ) as unknown as {
      values: [{ effective: { value: unknown } }];
    };
    oversized.values[0].effective.value = "x".repeat(200);
    expect(() =>
      assertCompatibleSettingsScopeSnapshot(oversized, 64),
    ).toThrow(
      expect.objectContaining({
        code: "opaque_value_too_large",
      }),
    );
  });
});

function incompatible(action: () => void): void {
  expect(action).toThrow(SettingsProtocolIncompatibilityError);
}
