import { describe, expect, test } from "bun:test";

import {
  BRIDGE_PROTOCOL_VERSION,
  BridgeJobTracker,
  BridgeProtocolValidationError,
  assertBridgeProtocolVersion,
  parseBridgeCancellationReceipt,
  parseBridgeCancellationRequest,
  parseBridgeCommandEnvelope,
  parseBridgeCommandReply,
  parseBridgeDeduplicationSupport,
  parseBridgeEventEnvelope,
  parseBridgeHelloRequest,
  parseBridgeJobTerminalEvent,
  parseBridgeNegotiationReceipt,
  parseBridgeProgressEvent,
  parseBridgeQueryEnvelope,
  parseBridgeQueryReply,
  parseBridgeSnapshotEnvelope,
  record,
} from "@inflatable-cookie/longhorn/bridge";

import {
  commandPayloadCodec,
  failureDetailCodec,
  fixture,
  jsonCodec,
  queryPayloadCodec,
  successPayloadCodec,
  values,
} from "./support.ts";

describe("Rust bridge protocol fixture", () => {
  test("validates every generated payload category", () => {
    assertBridgeProtocolVersion(fixture.protocolVersion);
    expect(fixture.protocolVersion).toBe(BRIDGE_PROTOCOL_VERSION);
    const hello = parseBridgeHelloRequest(fixture.hello);
    expect<unknown>(parseBridgeNegotiationReceipt(fixture.negotiation, hello)).toEqual(
      fixture.negotiation,
    );

    for (const request of values(fixture.queryRequests)) {
      expect(parseBridgeQueryEnvelope(request, queryPayloadCodec) as unknown).toEqual(request);
    }
    for (const reply of values(fixture.queryReplies)) {
      expect(
        parseBridgeQueryReply(reply, successPayloadCodec, failureDetailCodec) as unknown,
      ).toEqual(reply);
    }
    for (const request of values(fixture.commandRequests)) {
      expect(parseBridgeCommandEnvelope(request, commandPayloadCodec) as unknown).toEqual(request);
    }
    for (const reply of values(fixture.commandReplies)) {
      expect(
        parseBridgeCommandReply(reply, successPayloadCodec, failureDetailCodec) as unknown,
      ).toEqual(reply);
    }
    expect(parseBridgeSnapshotEnvelope(fixture.snapshot, successPayloadCodec) as unknown).toEqual(
      fixture.snapshot,
    );
    for (const event of values(fixture.events)) {
      expect(parseBridgeEventEnvelope(event, commandPayloadCodec) as unknown).toEqual(event);
    }
    expect(parseBridgeProgressEvent(fixture.progress, jsonCodec) as unknown).toEqual(
      fixture.progress,
    );
    expect<unknown>(parseBridgeCancellationRequest(fixture.cancellationRequest)).toEqual(
      fixture.cancellationRequest,
    );
    expect(
      parseBridgeCancellationReceipt(fixture.cancellationReceipt, failureDetailCodec) as unknown,
    ).toEqual(fixture.cancellationReceipt);
    expect(
      parseBridgeJobTerminalEvent(
        fixture.terminal,
        successPayloadCodec,
        failureDetailCodec,
      ) as unknown,
    ).toEqual(fixture.terminal);
  });

  test("fails closed on future versions, states, authority, and outcomes", () => {
    const incompatible = record(fixture.incompatibility, [
      "futureProtocolVersion",
      "unknownConnectionState",
      "unknownRetryClass",
      "unknownAuthorityShape",
      "unknownQueryOutcome",
      "unknownCommandOutcome",
      "unknownTerminalOutcome",
    ]);
    expectCode(
      () => assertBridgeProtocolVersion(incompatible.futureProtocolVersion),
      "unsupported_protocol_version",
    );

    const state = structuredClone(fixture.negotiation);
    (state as any).connection.state = incompatible.unknownConnectionState;
    expectCode(() => parseBridgeNegotiationReceipt(state), "unknown_connection_state");
    const authority = structuredClone(fixture.negotiation);
    (authority as any).domainAuthorities = [incompatible.unknownAuthorityShape];
    expectCode(() => parseBridgeNegotiationReceipt(authority), "unknown_read_authority");

    const query = structuredClone(values(fixture.queryReplies)[0]);
    (query as any).outcome = incompatible.unknownQueryOutcome;
    expectCode(
      () => parseBridgeQueryReply(query, successPayloadCodec, failureDetailCodec),
      "unknown_query_outcome",
    );
    const command = structuredClone(values(fixture.commandReplies)[0]);
    (command as any).outcome = incompatible.unknownCommandOutcome;
    expectCode(
      () => parseBridgeCommandReply(command, successPayloadCodec, failureDetailCodec),
      "unknown_command_outcome",
    );
    const terminal = structuredClone(fixture.terminal);
    (terminal as any).outcome = incompatible.unknownTerminalOutcome;
    expectCode(
      () => parseBridgeJobTerminalEvent(terminal, successPayloadCodec, failureDetailCodec),
      "unknown_job_outcome",
    );
  });

  test("rejects unknown retry and deduplication shapes", () => {
    const incompatible = record(fixture.incompatibility, [
      "futureProtocolVersion",
      "unknownConnectionState",
      "unknownRetryClass",
      "unknownAuthorityShape",
      "unknownQueryOutcome",
      "unknownCommandOutcome",
      "unknownTerminalOutcome",
    ]);
    const failed = structuredClone(values(fixture.queryReplies)[1]);
    (failed as any).outcome.rejected.retryClass = incompatible.unknownRetryClass;
    expectCode(
      () => parseBridgeQueryReply(failed, successPayloadCodec, failureDetailCodec),
      "unknown_retry_class",
    );
    expectCode(
      () => parseBridgeDeduplicationSupport({ future: 8 }),
      "unknown_deduplication_support",
    );
  });
});

function expectCode(run: () => unknown, code: BridgeProtocolValidationError["code"]): void {
  try {
    run();
    throw new Error("expected incompatibility");
  } catch (error) {
    expect(error).toBeInstanceOf(BridgeProtocolValidationError);
    expect((error as BridgeProtocolValidationError).code).toBe(code);
  }
}

test("job admission reads the request id first and never decodes the payload", () => {
  const tracker = new BridgeJobTracker("request:scan", "job:scan");
  const own = {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: { percent: 10, unknownField: true },
  };
  expect(tracker.admits(own)).toBe(true);
  expect(tracker.admits({ ...own, jobId: "job:other" })).toBe(false);
  // A foreign request is refused without reading its job id.
  expect(tracker.admits({ ...own, requestId: "request:other", jobId: "Not An Id!" })).toBe(false);
  // Under this job's own request, an unreadable job id is malformed.
  expectCode(() => tracker.admits({ ...own, jobId: "Not An Id!" }), "invalid_id");
  expectCode(() => tracker.admits(null), "invalid_object");
  expectCode(() => tracker.admits({ jobId: "job:scan" }), "invalid_id");
});
