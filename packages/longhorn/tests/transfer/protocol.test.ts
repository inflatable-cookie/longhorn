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
  TransferProtocolIncompatibilityError,
  assertCompatiblePanelTransferResponse,
  assertCompatibleTransferAbort,
  assertCompatibleTransferCancelResponse,
  assertCompatibleTransferCommitSelector,
  assertCompatibleTransferLeaseResponse,
  assertCompatibleTransferSessionResponse,
  assertCompatibleTransferTargetBinding,
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
        assertCompatibleTransferTargetBinding(target);
        return target.kind;
      }),
    );
    const selectorKinds = array(fixture.commit_requests).map((request) => {
      const selector = record(record(request).selector);
      assertCompatibleTransferCommitSelector(selector);
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
      assertCompatibleTransferAbort(abortValue);
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
      assertCompatibleTransferSessionResponse(response);
    }
    for (const response of array(fixture.lease_responses)) {
      assertCompatibleTransferLeaseResponse(response);
    }
    for (const response of array(fixture.commit_responses)) {
      assertCompatiblePanelTransferResponse(response);
    }
    for (const response of array(fixture.cancel_responses)) {
      assertCompatibleTransferCancelResponse(response);
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
        assertCompatibleTransferTargetBinding(
          incompatibility.unknown_target_binding,
        ),
      () =>
        assertCompatibleTransferCommitSelector(
          incompatibility.unknown_commit_selector,
        ),
      () =>
        assertCompatibleTransferSessionResponse(
          incompatibility.unknown_response_status,
        ),
      () =>
        assertCompatibleTransferAbort({
          protocol_version: 1,
          request_id: "request:future-domain",
          source: incompatibility.unknown_abort_domain,
          message: "future",
          retryable: false,
          session_consumed: false,
        }),
      () =>
        assertCompatibleTransferAbort({
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
        assertCompatibleTransferAbort({
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
      expect(check).toThrow(TransferProtocolIncompatibilityError);
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
