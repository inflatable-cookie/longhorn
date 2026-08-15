import { describe, expect, test } from "bun:test";

import {
  LicenceValidationError,
  assertLicenceActivateCommand,
  assertLicenceChangedEvent,
  assertLicenceDeactivateCommand,
  assertLicenceOutcome,
  assertLicenceRefreshCommand,
  assertLicenceReleaseSeatCommand,
  assertLicenceRenameSeatCommand,
  assertLicenceSnapshot,
} from "../../src/licence/validation.ts";
import { clone, fixture } from "./support.ts";

describe("Rust-generated licence protocol", () => {
  test("accepts the complete metadata-only golden fixture", () => {
    const value = fixture();
    assertLicenceSnapshot(value.snapshot);
    assertLicenceSnapshot(value.graceSnapshot);
    assertLicenceSnapshot(value.unlicensedSnapshot);
    value.activateCommands.forEach(assertLicenceActivateCommand);
    assertLicenceDeactivateCommand(value.deactivateCommand);
    assertLicenceRefreshCommand(value.refreshCommand);
    assertLicenceReleaseSeatCommand(value.releaseSeatCommand);
    assertLicenceRenameSeatCommand(value.renameSeatCommand);
    value.outcomes.forEach(assertLicenceOutcome);
    assertLicenceChangedEvent(value.changedEvent);
  });

  test("carries one rejection per protocol rejection code", () => {
    const value = fixture();
    const codes: string[] = value.outcomes.flatMap((outcome) =>
      outcome.status === "rejected" ? [outcome.code] : [],
    );
    expect([...codes].sort()).toEqual(
      [
        "clockRefused",
        "malformed",
        "noSeatsFree",
        "notRecognised",
        "revoked",
        "seatNotFound",
        "staleAuthority",
        "unreachable",
      ].sort(),
    );
    expect(value.outcomes.some((outcome) => outcome.status === "committed")).toBe(true);
  });

  test("rejects future versions, variants, fields, and credential material", () => {
    const value = fixture();
    const future = clone(value.snapshot) as unknown as Record<string, unknown>;
    future.protocolVersion = value.incompatibility.futureProtocolVersion;
    expect(() => assertLicenceSnapshot(future)).toThrow(LicenceValidationError);

    const unknownUsability = clone(value.snapshot);
    (unknownUsability.licence as { usability: { state: unknown } }).usability.state =
      value.incompatibility.unknownUsabilityState;
    expect(() => assertLicenceSnapshot(unknownUsability)).toThrow();

    const unknownStatus = clone(value.outcomes[0]) as unknown as Record<string, unknown>;
    unknownStatus.status = value.incompatibility.unknownOutcomeStatus;
    expect(() => assertLicenceOutcome(unknownStatus)).toThrow();

    const unknownCode = clone(value.outcomes[1]) as unknown as Record<string, unknown>;
    unknownCode.code = value.incompatibility.unknownRejectionCode;
    expect(() => assertLicenceOutcome(unknownCode)).toThrow();

    const unknownCredential = clone(value.activateCommands[0]) as unknown as Record<
      string,
      unknown
    >;
    (unknownCredential.credential as Record<string, unknown>).kind =
      value.incompatibility.unknownCredentialKind;
    expect(() => assertLicenceActivateCommand(unknownCredential)).toThrow();

    const extra = clone(value.changedEvent) as unknown as Record<string, unknown>;
    extra.durable = true;
    expect(() => assertLicenceChangedEvent(extra)).toThrow(/unexpected keys/);

    // The rule the licence protocol is built on: nothing that looks like
    // credential material comes back in a projection.
    const leaked = clone(value.snapshot) as unknown as Record<string, unknown>;
    leaked.keyId = "signing-key-fixture";
    expect(() => assertLicenceSnapshot(leaked)).toThrow(
      /credential material must not appear/,
    );
  });

  test("rejects unsafe numeric and zero authority values", () => {
    const value = fixture();
    const zeroEpoch = clone(value.snapshot);
    zeroEpoch.authorityEpoch = 0;
    expect(() => assertLicenceSnapshot(zeroEpoch)).toThrow(/positive safe integer/);

    const unsafe = clone(value.changedEvent);
    unsafe.authorityEpoch = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertLicenceChangedEvent(unsafe)).toThrow(/positive safe integer/);
  });
});
