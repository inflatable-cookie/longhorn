import { expect, test } from "bun:test";

import {
  BridgeJobTracker,
  classifyBridgeCommandRetry,
  classifyBridgeQueryRetry,
  parseBridgeCommandEnvelope,
  parseBridgeJobTerminalEvent,
  parseBridgeQueryEnvelope,
  parseBridgeStreamCursor,
  record,
  type BridgeStreamCursor,
} from "@inflatable-cookie/longhorn/bridge";
import { BridgeStreamTracker } from "@inflatable-cookie/longhorn/bridge/stream";

import {
  commandPayloadCodec,
  failureDetailCodec,
  fixture,
  queryPayloadCodec,
  successPayloadCodec,
  values,
} from "./support.ts";

test("TypeScript reproduces the Rust semantic trace exactly", () => {
  const query = parseBridgeQueryEnvelope(
    values(fixture.queryRequests)[0],
    queryPayloadCodec,
  );
  const command = parseBridgeCommandEnvelope(
    values(fixture.commandRequests)[0],
    commandPayloadCodec,
  );
  const uncertain = parseBridgeCommandEnvelope(
    values(fixture.commandRequests)[1],
    commandPayloadCodec,
  );
  const finite = { finite: 8 } as const;

  const listener = new BridgeStreamTracker(
    "session:fixture",
    "example.workspace",
  );
  const listenerFirst = [
    listener.classifyEvent(cursor(3, 9)),
    listener.acceptSnapshot(cursor(3, 8)),
    listener.acceptSnapshot(cursor(3, 9)),
  ];

  const ordered = new BridgeStreamTracker(
    "session:fixture",
    "example.workspace",
  );
  const orderedStream = [
    ordered.acceptSnapshot(cursor(3, 8)),
    ordered.classifyEvent(cursor(3, 8)),
    ordered.classifyEvent(cursor(3, 7)),
    ordered.classifyEvent(cursor(3, 9)),
    ordered.classifyEvent(cursor(3, 11)),
    ordered.classifyEvent(cursor(4, 0)),
  ];

  const terminal = parseBridgeJobTerminalEvent(
    fixture.terminal,
    successPayloadCodec,
    failureDetailCodec,
  );
  const jobs = new BridgeJobTracker("request:scan", "job:scan");
  const progress = {
    requestId: "request:scan",
    jobId: "job:scan",
    progress: { completed: 1 },
  };
  const foreign = {
    requestId: "request:other",
    jobId: "job:scan",
    progress: { completed: 2 },
  };

  const actual = {
    command_retry: [
      classifyBridgeCommandRetry(
        command,
        "uncertain",
        "afterReconnect",
        finite,
      ),
      classifyBridgeCommandRetry(
        uncertain,
        "uncertain",
        "afterReconnect",
        finite,
      ),
      classifyBridgeCommandRetry(
        command,
        "notDispatched",
        "afterReconnect",
        finite,
      ),
    ],
    query_retry: [
      classifyBridgeQueryRetry(query, "afterBackoff", true),
      classifyBridgeQueryRetry(query, "afterBackoff", false),
      classifyBridgeQueryRetry(query, "never", true),
    ],
    listener_first: listenerFirst,
    ordered_stream: orderedStream,
    job: [
      jobs.classifyProgress(progress),
      jobs.classifyProgress(foreign),
      jobs.classifyTerminal(terminal),
      jobs.classifyProgress(progress),
      jobs.classifyTerminal(terminal),
    ],
  };

  expect(actual as unknown).toEqual(record(fixture.semanticTrace, [
    "command_retry",
    "query_retry",
    "listener_first",
    "ordered_stream",
    "job",
  ]));
});

function cursor(epoch: number, sequence: number): BridgeStreamCursor {
  return parseBridgeStreamCursor({
    sessionId: "session:fixture",
    domainId: "example.workspace",
    authorityEpoch: epoch,
    sequence,
  });
}
