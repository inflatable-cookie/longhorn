import { describe, expect, test } from "bun:test";

import {
  UpdateValidationError,
  assertUpdateApplyCommand,
  assertUpdateCancelCommand,
  assertUpdateChangedEvent,
  assertUpdateCheckCommand,
  assertUpdateDeferCommand,
  assertUpdateInstallAuthorization,
  assertUpdateOutcome,
  assertUpdatePrepareCommand,
  assertUpdateProgressEvent,
  assertUpdateSelectChannelCommand,
  assertUpdateSnapshot,
} from "../../src/update/validation.ts";
import { clone, fixture } from "./support.ts";

describe("Rust-generated update protocol", () => {
  test("accepts the complete metadata-only golden fixture", () => {
    const value = fixture();
    assertUpdateSnapshot(value.snapshot);
    assertUpdateSnapshot(value.managedSnapshot);
    assertUpdateSnapshot(value.stagedSnapshot);
    assertUpdateSnapshot(value.aheadSnapshot);
    assertUpdateSnapshot(value.withheldSnapshot);
    assertUpdateSnapshot(value.upToDateSnapshot);
    assertUpdateCheckCommand(value.checkCommand);
    assertUpdateSelectChannelCommand(value.selectChannelCommand);
    assertUpdateDeferCommand(value.deferCommand);
    assertUpdatePrepareCommand(value.prepareCommand);
    assertUpdateApplyCommand(value.applyCommand);
    assertUpdateCancelCommand(value.cancelCommand);
    assertUpdateInstallAuthorization(value.authorizationHeld);
    assertUpdateInstallAuthorization(value.authorizationDeferred);
    value.outcomes.forEach(assertUpdateOutcome);
    assertUpdateChangedEvent(value.changedEvent);
    assertUpdateProgressEvent(value.progressEvent);
  });

  test("carries one rejection per protocol rejection code, channelMismatch included", () => {
    const value = fixture();
    const codes: string[] = value.outcomes.flatMap((outcome) =>
      outcome.status === "rejected" ? [outcome.code] : [],
    );
    expect([...codes].sort()).toEqual(
      [
        "channelMismatch",
        "installFailed",
        "noOffer",
        "notWritable",
        "signatureRejected",
        "staleAuthority",
        "unavailable",
        "unreachable",
      ].sort(),
    );
    expect(value.outcomes.some((outcome) => outcome.status === "committed")).toBe(true);
  });

  /**
   * The retained artifact is its own identity: version, channel and digest.
   * A staged snapshot with no digest, or a digest that is not one, is a
   * surface claiming to know what it is holding when it does not.
   */
  test("a staged artifact must carry its version, channel and digest", () => {
    const value = fixture();
    const staged = value.stagedSnapshot.staged;
    expect(staged).not.toBeNull();
    expect(staged?.digest).toHaveLength(64);

    const shortDigest = clone(value.stagedSnapshot) as unknown as Record<string, unknown>;
    (shortDigest.staged as { digest: unknown }).digest = "abc";
    expect(() => assertUpdateSnapshot(shortDigest)).toThrow(/hex/);

    const unknownChannel = clone(value.stagedSnapshot) as unknown as Record<string, unknown>;
    (unknownChannel.staged as { channel: unknown }).channel = "canary";
    expect(() => assertUpdateSnapshot(unknownChannel)).toThrow();

    const missing = clone(value.stagedSnapshot) as unknown as Record<string, unknown>;
    delete (missing.staged as Record<string, unknown>).digest;
    expect(() => assertUpdateSnapshot(missing)).toThrow(/unexpected keys/);
  });

  /** The live channel carries a report, so a zero-byte report is valid and a
   * negative one is not. */
  test("a live progress event carries byte counts", () => {
    const value = fixture();
    assertUpdateProgressEvent(value.progressEvent);
    expect(value.progressEvent.progress.state).toBe("downloading");

    const negative = clone(value.progressEvent) as unknown as Record<string, unknown>;
    (negative.progress as { received: unknown }).received = -1;
    expect(() => assertUpdateProgressEvent(negative)).toThrow(/byte count/);

    const missing = clone(value.progressEvent) as unknown as Record<string, unknown>;
    delete (missing.progress as Record<string, unknown>).expected;
    expect(() => assertUpdateProgressEvent(missing)).toThrow(/unexpected keys/);
  });

  /**
   * A held lease has no reason to state and a deferral has nothing else: the
   * discriminator is the whole difference between "the barrier is held" and
   * "the barrier was refused".
   */
  test("the authorization projection is held or deferred with its cause", () => {
    const value = fixture();
    expect(value.authorizationHeld.status).toBe("held");
    expect("cause" in value.authorizationHeld).toBe(false);
    expect(value.authorizationDeferred.status).toBe("deferred");
    if (value.authorizationDeferred.status !== "deferred") throw new Error("unreachable");
    expect(value.authorizationDeferred.cause.cause).toBe("workInFlight");

    const stale = clone(value.authorizationHeld) as unknown as Record<string, unknown>;
    stale.status = value.incompatibility.unknownInstallAuthorizationStatus;
    expect(() => assertUpdateInstallAuthorization(stale)).toThrow();

    const missingCause = clone(value.authorizationDeferred) as unknown as Record<string, unknown>;
    delete missingCause.cause;
    expect(() => assertUpdateInstallAuthorization(missingCause)).toThrow(/unexpected keys/);
  });

  test("rejects future versions, variants, fields, and product payloads", () => {
    const value = fixture();
    const future = clone(value.snapshot) as unknown as Record<string, unknown>;
    future.protocolVersion = value.incompatibility.futureProtocolVersion;
    expect(() => assertUpdateSnapshot(future)).toThrow(UpdateValidationError);

    const unknownChannel = clone(value.snapshot);
    (unknownChannel as { channel: unknown }).channel = value.incompatibility.unknownChannel;
    expect(() => assertUpdateSnapshot(unknownChannel)).toThrow();

    const unknownAvailability = clone(value.snapshot);
    (unknownAvailability.availability as { state: unknown }).state =
      value.incompatibility.unknownAvailabilityState;
    expect(() => assertUpdateSnapshot(unknownAvailability)).toThrow();

    const unknownStatus = clone(value.outcomes[0]) as unknown as Record<string, unknown>;
    unknownStatus.status = value.incompatibility.unknownOutcomeStatus;
    expect(() => assertUpdateOutcome(unknownStatus)).toThrow();

    const unknownCode = clone(value.outcomes[1]) as unknown as Record<string, unknown>;
    unknownCode.code = value.incompatibility.unknownRejectionCode;
    expect(() => assertUpdateOutcome(unknownCode)).toThrow();

    const extra = clone(value.changedEvent) as unknown as Record<string, unknown>;
    extra.durable = true;
    expect(() => assertUpdateChangedEvent(extra)).toThrow(/unexpected keys/);

    const payload = clone(value.snapshot) as unknown as Record<string, unknown>;
    payload.payload = { product: "forbidden" };
    expect(() => assertUpdateSnapshot(payload)).toThrow(/product payload field is forbidden/);
  });

  test("rejects unsafe numeric and zero authority values", () => {
    const value = fixture();
    const zeroEpoch = clone(value.snapshot);
    zeroEpoch.authorityEpoch = 0;
    expect(() => assertUpdateSnapshot(zeroEpoch)).toThrow(/positive safe integer/);

    const unsafe = clone(value.changedEvent);
    unsafe.authorityEpoch = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertUpdateChangedEvent(unsafe)).toThrow(/positive safe integer/);
  });
});
