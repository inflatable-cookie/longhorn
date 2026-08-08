import { describe, expect, test } from "bun:test";

import {
  BridgeDomainClient,
  BridgeHostRouter,
  BridgeSerializationError,
  BridgeSessionClient,
  DirectBridgeAdapter,
  SerializedLoopbackBridgeAdapter,
  bridgeCodec,
  parseBridgeCommandEnvelope,
  parseBridgeCommandReply,
  parseBridgeHelloRequest,
  parseBridgeNegotiationReceipt,
  parseBridgeQueryEnvelope,
  parseBridgeQueryReply,
  type BridgeCommandEnvelope,
  type BridgeCommandReply,
  type BridgeOperationDescriptor,
  type BridgeQueryEnvelope,
  type BridgeQueryReply,
} from "@inflatable-cookie/longhorn/bridge";

import {
  commandPayloadCodec,
  failureDetailCodec,
  fixture,
  jsonCodec,
  queryPayloadCodec,
  successPayloadCodec,
  type CommandPayload,
  type FailureDetail,
  type QueryPayload,
  type SuccessPayload,
  values,
} from "./support.ts";

type QueryRequest = BridgeQueryEnvelope<QueryPayload>;
type QueryReply = BridgeQueryReply<SuccessPayload, FailureDetail>;
type CommandRequest = BridgeCommandEnvelope<CommandPayload>;
type CommandReply = BridgeCommandReply<SuccessPayload, FailureDetail>;

const queryOperation: BridgeOperationDescriptor<QueryRequest, QueryReply> = {
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

const commandOperation: BridgeOperationDescriptor<
  CommandRequest,
  CommandReply
> = {
  route: "example.workspace.mutate",
  domainId: "example.workspace",
  kind: "command",
  requiredCapability: "mutate",
  request: bridgeCodec((value) =>
    parseBridgeCommandEnvelope(value, commandPayloadCodec)
  ),
  reply: bridgeCodec((value) =>
    parseBridgeCommandReply(value, successPayloadCodec, failureDetailCodec)
  ),
};

describe("direct and serialized loopback conformance", () => {
  test("produce the same negotiated operation trace", async () => {
    const router = fixtureRouter();
    const direct = new DirectBridgeAdapter(router);
    const loopback = new SerializedLoopbackBridgeAdapter(router);

    const [directTrace, loopbackTrace] = await Promise.all([
      runTrace(direct),
      runTrace(loopback),
    ]);
    expect(loopbackTrace).toEqual(directTrace);
    expect(directTrace).toEqual({
      host: "host:fixture",
      query: values(fixture.queryReplies)[0],
      command: values(fixture.commandReplies)[0],
    });
  });

  test("surfaces request and reply serialization failures at the boundary", async () => {
    const router = fixtureRouter();
    const loopback = new SerializedLoopbackBridgeAdapter(router);
    const requestFailure = rawOperation("fixture.serialization.request");
    router.register(requestFailure, (value) => value);

    await expect(
      loopback.invoke(requestFailure, { impossible: 1n }),
    ).rejects.toMatchObject({
      name: "BridgeSerializationError",
      phase: "operation_request",
    });

    const replyFailure = rawOperation("fixture.serialization.reply");
    router.register(replyFailure, () => {
      const circular: Record<string, unknown> = {};
      circular.self = circular;
      return circular;
    });
    try {
      await loopback.invoke(replyFailure, {});
      throw new Error("expected reply serialization failure");
    } catch (error) {
      expect(error).toBeInstanceOf(BridgeSerializationError);
      expect((error as BridgeSerializationError).phase)
        .toBe("operation_reply");
    }
  });
});

function fixtureRouter(): BridgeHostRouter {
  const hello = parseBridgeHelloRequest(fixture.hello);
  const negotiation = parseBridgeNegotiationReceipt(
    fixture.negotiation,
    hello,
  );
  const router = new BridgeHostRouter(() => negotiation);
  router.register(queryOperation, (request) => {
    expect(request.context.domainId).toBe("example.workspace");
    return values(fixture.queryReplies)[0] as QueryReply;
  });
  router.register(commandOperation, (request) => {
    expect(request.authorityEpoch).toBe(3);
    return values(fixture.commandReplies)[0] as CommandReply;
  });
  return router;
}

async function runTrace(
  adapter: DirectBridgeAdapter | SerializedLoopbackBridgeAdapter,
): Promise<Record<string, unknown>> {
  const session = await new BridgeSessionClient(adapter).connect(
    parseBridgeHelloRequest(fixture.hello),
  );
  const client = new BridgeDomainClient(session, adapter);
  return {
    host: session.receipt.host.hostInstanceId,
    query: await client.execute(
      queryOperation,
      values(fixture.queryRequests)[0] as QueryRequest,
    ),
    command: await client.execute(
      commandOperation,
      values(fixture.commandRequests)[0] as CommandRequest,
    ),
  };
}

function rawOperation(
  route: string,
): BridgeOperationDescriptor<unknown, unknown> {
  return {
    route,
    domainId: "example.workspace",
    kind: "query",
    requiredCapability: "query",
    request: jsonCodec,
    reply: jsonCodec,
  };
}
