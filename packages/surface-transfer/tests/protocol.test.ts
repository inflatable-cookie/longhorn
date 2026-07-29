import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import {
  SURFACE_SESSION_RESPONSE_STATUSES,
  SURFACE_TRANSFER_ABORT_DOMAINS,
  SURFACE_TRANSFER_ERROR_CODES,
  SURFACE_TRANSFER_RESPONSE_STATUSES,
  SURFACE_TRANSFER_TARGET_KINDS,
  SurfaceTransferProtocolIncompatibilityError,
  assertCompatibleSurfaceSessionResponse,
  assertCompatibleSurfaceTransferAbort,
  assertCompatibleSurfaceTransferResponse,
  assertCompatibleSurfaceTransferTarget,
} from "@longhorn/surface-transfer";
import {
  TransferProtocolIncompatibilityError,
  assertTransferProtocolVersion,
} from "@longhorn/transfer";

const fixturePath = new URL(
  "../../../fixtures/surface-transfer/protocol-v1.json",
  import.meta.url,
);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

describe("Rust Surface transfer protocol fixture", () => {
  test("round-trips every generated category without changing JSON", () => {
    assertTransferProtocolVersion(fixture.protocol_version);
    for (const category of [
      "session_requests",
      "commit_requests",
      "session_responses",
      "commit_responses",
      "aborts",
    ]) {
      expect(roundTrip(fixture[category])).toEqual(fixture[category]);
    }
  });

  test("covers every target, response, abort, and adapter error variant", () => {
    const targets = array(fixture.commit_responses)
      .map(record)
      .filter((response) => response.status === "committed")
      .map((response) => {
        const target = record(record(response.completion).target);
        assertCompatibleSurfaceTransferTarget(target);
        return target.kind;
      });
    expect(new Set(targets)).toEqual(
      new Set(SURFACE_TRANSFER_TARGET_KINDS),
    );
    expect(statuses("session_responses")).toEqual(
      new Set(SURFACE_SESSION_RESPONSE_STATUSES),
    );
    expect(statuses("commit_responses")).toEqual(
      new Set(SURFACE_TRANSFER_RESPONSE_STATUSES),
    );

    const errorCodes = new Set<unknown>();
    const domains = new Set<unknown>();
    for (const value of array(fixture.aborts)) {
      assertCompatibleSurfaceTransferAbort(value);
      const source = record(record(value).source);
      domains.add(source.domain);
      if (source.domain === "surface_transfer") {
        errorCodes.add(source.code);
      }
    }
    expect(domains).toEqual(new Set(SURFACE_TRANSFER_ABORT_DOMAINS));
    expect(errorCodes).toEqual(new Set(SURFACE_TRANSFER_ERROR_CODES));

    for (const response of array(fixture.session_responses)) {
      assertCompatibleSurfaceSessionResponse(response);
    }
    for (const response of array(fixture.commit_responses)) {
      assertCompatibleSurfaceTransferResponse(response);
    }
  });
});

describe("Surface transfer incompatibility", () => {
  const incompatibility = record(fixture.incompatibility);

  test("rejects future versions and unknown variants", () => {
    expect(() =>
      assertTransferProtocolVersion(incompatibility.future_protocol_version),
    ).toThrow(TransferProtocolIncompatibilityError);

    for (const check of [
      () =>
        assertCompatibleSurfaceTransferTarget(
          incompatibility.unknown_target,
        ),
      () =>
        assertCompatibleSurfaceSessionResponse(
          incompatibility.unknown_response_status,
        ),
      () =>
        assertCompatibleSurfaceTransferAbort({
          protocol_version: 1,
          request_id: "request:future-domain",
          source: incompatibility.unknown_abort_domain,
          surface_code: null,
          message: "future",
          retryable: false,
          session_consumed: false,
          reconciliation_required: false,
        }),
      () =>
        assertCompatibleSurfaceTransferAbort({
          protocol_version: 1,
          request_id: "request:future-code",
          source: {
            domain: "surface_transfer",
            code: incompatibility.unknown_error_code,
          },
          surface_code: null,
          message: "future",
          retryable: false,
          session_consumed: false,
          reconciliation_required: false,
        }),
    ]) {
      expect(check).toThrow(SurfaceTransferProtocolIncompatibilityError);
    }
  });
});

function statuses(category: string): Set<unknown> {
  return new Set(
    array(fixture[category]).map((response) => record(response).status),
  );
}

function roundTrip(value: unknown): unknown {
  return JSON.parse(JSON.stringify(value));
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("expected JSON object");
  }
  return value as Record<string, unknown>;
}

function array(value: unknown): unknown[] {
  if (!Array.isArray(value)) {
    throw new TypeError("expected JSON array");
  }
  return value;
}
