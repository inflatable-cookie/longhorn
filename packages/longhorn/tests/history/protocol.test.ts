import { describe, expect, test } from "bun:test";

import {
  HistoryProtocolCompatibilityError,
  assertCompatibleHistoryChangedEvent,
  assertCompatibleHistoryNavigationCommand,
  assertCompatibleHistoryNavigationResult,
  assertCompatibleHistoryPageCommand,
  assertCompatibleHistoryPageSnapshot,
  assertCompatibleHistorySnapshot,
} from "../../src/history/validation.ts";
import { clone, fixture } from "./support.ts";

describe("Rust-generated history protocol", () => {
  test("accepts the complete metadata-only golden fixture", () => {
    const value = fixture();
    assertCompatibleHistorySnapshot(value.snapshot);
    assertCompatibleHistoryPageCommand(value.pageRequest);
    assertCompatibleHistoryPageSnapshot(value.page);
    assertCompatibleHistoryNavigationCommand(value.navigationCommand);
    value.navigationResults.forEach(assertCompatibleHistoryNavigationResult);
    assertCompatibleHistoryChangedEvent(value.changedEvent);
  });

  test("rejects future versions, variants, fields, and product payloads", () => {
    const value = fixture();
    const future = clone(value.snapshot) as unknown as Record<string, unknown>;
    future.protocolVersion = value.incompatibility.futureProtocolVersion;
    expect(() => assertCompatibleHistorySnapshot(future)).toThrow(
      HistoryProtocolCompatibilityError,
    );

    const unknownMode = clone(value.snapshot);
    (unknownMode.summary as { mode: unknown }).mode =
      value.incompatibility.unknownMode;
    expect(() => assertCompatibleHistorySnapshot(unknownMode)).toThrow();

    const unknownStatus = clone(value.navigationResults[0]) as unknown as Record<
      string,
      unknown
    >;
    unknownStatus.status = value.incompatibility.unknownNavigationStatus;
    expect(() => assertCompatibleHistoryNavigationResult(unknownStatus)).toThrow();

    const payload = clone(value.page) as unknown as Record<string, unknown>;
    payload.payload = { product: "forbidden" };
    expect(() => assertCompatibleHistoryPageSnapshot(payload)).toThrow(
      /product payload is forbidden/,
    );

    const extra = clone(value.changedEvent) as unknown as Record<string, unknown>;
    extra.durable = true;
    expect(() => assertCompatibleHistoryChangedEvent(extra)).toThrow(
      /unknown field/,
    );
  });

  test("rejects unsafe numeric and zero authority values", () => {
    const value = fixture();
    const zeroEpoch = clone(value.snapshot);
    zeroEpoch.authorityEpoch = 0;
    expect(() => assertCompatibleHistorySnapshot(zeroEpoch)).toThrow(
      /nonzero/,
    );

    const unsafe = clone(value.page);
    unsafe.totalEntries = Number.MAX_SAFE_INTEGER + 1;
    expect(() => assertCompatibleHistoryPageSnapshot(unsafe)).toThrow(
      /safe integer/,
    );
  });

  test("matches the Rust opaque-id grammar without requiring a namespace colon", () => {
    const value = fixture();
    value.snapshot.summary.currentEntryId = "history-0001";
    assertCompatibleHistorySnapshot(value.snapshot);

    value.snapshot.summary.currentEntryId = "History-0001";
    expect(() => assertCompatibleHistorySnapshot(value.snapshot)).toThrow(
      /bounded lowercase opaque id/,
    );
  });
});
