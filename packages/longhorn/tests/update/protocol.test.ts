import { describe, expect, test } from "bun:test";

import {
  UpdateValidationError,
  assertUpdateChangedEvent,
  assertUpdateCheckCommand,
  assertUpdateDeferCommand,
  assertUpdateInstallCommand,
  assertUpdateOutcome,
  assertUpdateSelectChannelCommand,
  assertUpdateSnapshot,
} from "../../src/update/validation.ts";
import { clone, fixture } from "./support.ts";

describe("Rust-generated update protocol", () => {
  test("accepts the complete metadata-only golden fixture", () => {
    const value = fixture();
    assertUpdateSnapshot(value.snapshot);
    assertUpdateSnapshot(value.managedSnapshot);
    assertUpdateSnapshot(value.aheadSnapshot);
    assertUpdateSnapshot(value.withheldSnapshot);
    assertUpdateSnapshot(value.upToDateSnapshot);
    assertUpdateCheckCommand(value.checkCommand);
    assertUpdateSelectChannelCommand(value.selectChannelCommand);
    assertUpdateDeferCommand(value.deferCommand);
    assertUpdateInstallCommand(value.installCommand);
    value.outcomes.forEach(assertUpdateOutcome);
    assertUpdateChangedEvent(value.changedEvent);
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
