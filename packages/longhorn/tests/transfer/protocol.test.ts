import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import {
  PANEL_TRANSFER_ERROR_CODES,
  PANEL_TRANSFER_RESPONSE_STATUSES,
  TRANSFER_ABORT_DOMAINS,
  TRANSFER_CANCEL_RESPONSE_STATUSES,
  TRANSFER_COMMIT_SELECTOR_KINDS,
  TRANSFER_ERROR_CODES,
  TRANSFER_LEASE_RESPONSE_STATUSES,
  TRANSFER_SESSION_RESPONSE_STATUSES,
  TRANSFER_TARGET_BINDING_KINDS,
  TransferProtocolValidationError,
  assertValidPanelTransferResponse,
  assertValidTransferAbort,
  assertValidTransferCancelResponse,
  assertValidTransferCommitSelector,
  assertValidTransferLeaseResponse,
  assertValidTransferSessionResponse,
  assertValidTransferTargetBinding,
  assertTransferProtocolVersion,
} from "@inflatable-cookie/longhorn/transfer";

const fixturePath = new URL(
  "../../../../fixtures/transfer/protocol-v1.json",
  import.meta.url,
);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

describe("Rust transfer protocol fixture", () => {
  test("round-trips every generated category without changing JSON", () => {
    assertTransferProtocolVersion(fixture.protocol_version);
    for (const category of [
      "client_snapshot",
      "session_requests",
      "lease_requests",
      "commit_requests",
      "cancel_requests",
      "session_responses",
      "lease_responses",
      "commit_responses",
      "cancel_responses",
      "aborts",
    ]) {
      expect(roundTrip(fixture[category])).toEqual(fixture[category]);
    }
  });

  test("covers every target, selector, response, abort, and error variant", () => {
    const targetKinds = array(fixture.lease_requests).flatMap((request) =>
      array(record(request).zones).map((zone) => {
        const target = record(record(zone).target);
        assertValidTransferTargetBinding(target);
        return target.kind;
      }),
    );
    const selectorKinds = array(fixture.commit_requests).map((request) => {
      const selector = record(record(request).selector);
      assertValidTransferCommitSelector(selector);
      return selector.kind;
    });
    expect(new Set(targetKinds)).toEqual(
      new Set(TRANSFER_TARGET_BINDING_KINDS),
    );
    expect(new Set(selectorKinds)).toEqual(
      new Set(TRANSFER_COMMIT_SELECTOR_KINDS),
    );

    expect(responseStatuses("session_responses")).toEqual(
      new Set(TRANSFER_SESSION_RESPONSE_STATUSES),
    );
    expect(responseStatuses("lease_responses")).toEqual(
      new Set(TRANSFER_LEASE_RESPONSE_STATUSES),
    );
    expect(responseStatuses("commit_responses")).toEqual(
      new Set(PANEL_TRANSFER_RESPONSE_STATUSES),
    );
    expect(responseStatuses("cancel_responses")).toEqual(
      new Set(TRANSFER_CANCEL_RESPONSE_STATUSES),
    );

    const errorCodes = new Map<string, Set<unknown>>();
    for (const abortValue of array(fixture.aborts)) {
      assertValidTransferAbort(abortValue);
      const source = record(record(abortValue).source);
      const codes = errorCodes.get(String(source.domain)) ?? new Set();
      codes.add(source.code);
      errorCodes.set(String(source.domain), codes);
    }
    expect(new Set(errorCodes.keys())).toEqual(
      new Set(TRANSFER_ABORT_DOMAINS),
    );
    expect(errorCodes.get("transfer")).toEqual(
      new Set(TRANSFER_ERROR_CODES),
    );
    expect(errorCodes.get("panel")).toEqual(
      new Set(PANEL_TRANSFER_ERROR_CODES),
    );

    for (const response of array(fixture.session_responses)) {
      assertValidTransferSessionResponse(response);
    }
    for (const response of array(fixture.lease_responses)) {
      assertValidTransferLeaseResponse(response);
    }
    for (const response of array(fixture.commit_responses)) {
      assertValidPanelTransferResponse(response);
    }
    for (const response of array(fixture.cancel_responses)) {
      assertValidTransferCancelResponse(response);
    }
  });
});

describe("transfer protocol incompatibility", () => {
  const incompatibility = record(fixture.incompatibility);

  test("rejects future versions and unknown variants", () => {
    for (const check of [
      () =>
        assertTransferProtocolVersion(
          incompatibility.future_protocol_version,
        ),
      () =>
        assertValidTransferTargetBinding(
          incompatibility.unknown_target_binding,
        ),
      () =>
        assertValidTransferCommitSelector(
          incompatibility.unknown_commit_selector,
        ),
      () =>
        assertValidTransferSessionResponse(
          incompatibility.unknown_response_status,
        ),
      () =>
        assertValidTransferAbort({
          protocol_version: 1,
          request_id: "request:future-domain",
          source: incompatibility.unknown_abort_domain,
          message: "future",
          retryable: false,
          session_consumed: false,
        }),
      () =>
        assertValidTransferAbort({
          protocol_version: 1,
          request_id: "request:future-transfer-code",
          source: {
            domain: "transfer",
            code: incompatibility.unknown_transfer_error_code,
          },
          message: "future",
          retryable: false,
          session_consumed: false,
        }),
      () =>
        assertValidTransferAbort({
          protocol_version: 1,
          request_id: "request:future-panel-code",
          source: {
            domain: "panel",
            code: incompatibility.unknown_panel_error_code,
          },
          message: "future",
          retryable: false,
          session_consumed: false,
        }),
    ]) {
      expect(check).toThrow(TransferProtocolValidationError);
    }
  });
});

function responseStatuses(category: string): Set<unknown> {
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
