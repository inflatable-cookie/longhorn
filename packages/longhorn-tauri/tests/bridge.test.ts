import { expect, test } from "bun:test";

import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";
import {
  bridgeCodec,
  parseBridgeQueryEnvelope,
  parseBridgeQueryReply,
  type BridgeOperationDescriptor,
  type BridgeQueryEnvelope,
  type BridgeQueryReply,
} from "@inflatable-cookie/longhorn/bridge";
import {
  BRIDGE_HELLO_COMMAND,
  BRIDGE_QUERY_COMMAND,
  connectTauriBridge,
} from "@inflatable-cookie/longhorn-tauri/bridge";

import {
  failureDetailCodec,
  fixture,
  queryPayloadCodec,
  successPayloadCodec,
  type FailureDetail,
  type QueryPayload,
  type SuccessPayload,
  values,
} from "../../longhorn/tests/bridge/support.ts";

type QueryRequest = BridgeQueryEnvelope<QueryPayload>;
type QueryReply = BridgeQueryReply<SuccessPayload, FailureDetail>;

const query: BridgeOperationDescriptor<QueryRequest, QueryReply> = {
  route: "example.workspace.query",
  domainId: "example.workspace",
  kind: "query",
  requiredCapability: "query",
  request: bridgeCodec((value) =>
    parseBridgeQueryEnvelope(value, queryPayloadCodec)
  ),
  reply: bridgeCodec((value) =>
    parseBridgeQueryReply(value, successPayloadCodec, failureDetailCodec)
  ),
};

test("composes checked bridge sessions over generic Tauri commands", async () => {
  const calls: Array<{
    command: string;
    arguments_: Record<string, unknown>;
  }> = [];
  const transport: InvokeTransport = {
    invoke(command, arguments_) {
      calls.push({ command, arguments_ });
      if (command === BRIDGE_HELLO_COMMAND) {
        return Promise.resolve(fixture.negotiation);
      }
      if (command === BRIDGE_QUERY_COMMAND) {
        return Promise.resolve(values(fixture.queryReplies)[0]);
      }
      throw new Error(`unexpected command: ${command}`);
    },
  };

  const connection = await connectTauriBridge(
    fixture.hello as never,
    transport,
  );
  const reply = await connection.domain.execute(
    query,
    values(fixture.queryRequests)[0] as QueryRequest,
  );

  expect(reply).toEqual(values(fixture.queryReplies)[0] as QueryReply);
  expect(calls.map(({ command }) => command)).toEqual([
    "longhorn_bridge_hello",
    "longhorn_bridge_query",
  ]);
  expect(calls[1]?.arguments_).toEqual({
    route: "example.workspace.query",
    request: values(fixture.queryRequests)[0],
  });
});
