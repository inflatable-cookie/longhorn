import { describe, expect, test } from "bun:test";

import {
  HistoryProtocolValidationError,
  assertValidHistoryChangedEvent,
  assertValidHistoryNavigationCommand,
  assertValidHistoryNavigationResult,
  assertValidHistoryPageCommand,
  assertValidHistoryPageSnapshot,
  assertValidHistorySnapshot,
} from "../../src/history/validation.ts";
import { clone, fixture } from "./support.ts";

describe("Rust-generated history protocol", () => {
  test("accepts the complete metadata-only golden fixture", () => {
    const value = fixture();
    assertValidHistorySnapshot(value.snapshot);
    assertValidHistoryPageCommand(value.pageRequest);
    assertValidHistoryPageSnapshot(value.page);
    assertValidHistoryNavigationCommand(value.navigationCommand);
    value.navigationResults.forEach(assertValidHistoryNavigationResult);
    assertValidHistoryChangedEvent(value.changedEvent);
  });

  test("rejects future versions, variants, fields, and product payloads", () => {
    const value = fixture();
    const future = clone(value.snapshot) as unknown as Record<string, unknown>;
    future.protocolVersion = value.incompatibility.futureProtocolVersion;
    expect(() => assertValidHistorySnapshot(future)).toThrow(
      HistoryProtocolValidationError,
    );

    const unknownMode = clone(value.snapshot);
    (unknownMode.summary as { mode: unknown }).mode =
      value.incompatibility.unknownMode;
    expect(() => assertValidHistorySnapshot(unknownMode)).toThrow();

    const unknownStatus = clone(value.navigationResults[0]) as unknown as Record<
      string,
      unknown
    >;
    unknownStatus.status = value.incompatibility.unknownNavigationStatus;
    expect(() => assertValidHistoryNavigationResult(unknownStatus)).toThrow();

    const payload = clone(value.page) as unknown as Record<string, unknown>;
    payload.payload = { product: "forbidden" };
    expect(() => assertValidHistoryPageSnapshot(payload)).toThrow(
      /product payload is forbidden/,
    );

    const extra = clone(value.changedEvent) as unknown as Record<string, unknown>;
    extra.durable = true;
    expect(() => assertValidHistoryChangedEvent(extra)).toThrow(
      /unknown field/,
    );
  });

  test("rejects unsafe numeric and zero authority values", () => {
    const value = fixture();
    const zeroEpoch = clone(value.snapshot);
    zeroEpoch.authorityEpoch = 0;
    expect(() => assertValidHistorySnapshot(zeroEpoch)).toThrow(
      /nonzero/,
    );

    const unsafe = clone(value.page);
    unsafe.totalEntries = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertValidHistoryPageSnapshot(unsafe)).toThrow(
      /safe integer/,
    );
  });

  test("matches the Rust opaque-id grammar without requiring a namespace colon", () => {
    const value = fixture();
    value.snapshot.summary.currentEntryId = "history-0001";
    assertValidHistorySnapshot(value.snapshot);

    value.snapshot.summary.currentEntryId = "History-0001";
    expect(() => assertValidHistorySnapshot(value.snapshot)).toThrow(
      /bounded lowercase opaque id/,
    );
  });
});
