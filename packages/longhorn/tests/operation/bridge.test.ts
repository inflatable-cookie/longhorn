import { describe, expect, test } from "bun:test";
import {
  BridgeDomainClient,
  BridgeSession,
  type BridgeOperationDescriptor,
} from "@inflatable-cookie/longhorn/bridge";

import {
  BridgeOperationJobCorrelation,
  createBridgeOperationPort,
} from "../../src/operation/bridge.ts";
import { OperationClient } from "../../src/operation/client.ts";
import { fixture } from "./support.ts";

describe("bridge operation composition", () => {
  test("converges with the operation trace while bridge ids remain correlation only", async () => {
    const session = operationSession();
    const adapter = {
      async invoke(operation: BridgeOperationDescriptor<unknown, unknown>, request: unknown) {
        const envelope = request as { context: { requestId: string }; payload: { requestId: string } };
        if (operation.route.endsWith("snapshot")) {
          return { requestId: envelope.context.requestId, outcome: { success: fixture.snapshotResponse } };
        }
        if (operation.route.endsWith("mutate")) {
          const result = fixture.mutationResults.find((candidate) => candidate.requestId === envelope.payload.requestId);
          return { requestId: envelope.context.requestId, authoritativeRevision: result?.snapshot.catalogueRevision ?? null, outcome: { applied: result } };
        }
        return { requestId: envelope.context.requestId, outcome: { success: fixture.cancellationResult } };
      },
    };
    let bridgeSequence = 0;
    const client = new OperationClient(createBridgeOperationPort({
      client: new BridgeDomainClient(session, adapter),
      domainId: "longhorn.operation",
      sessionId: "session:operation",
      authorityEpoch: 7,
      nextBridgeRequestId: () => `bridge-request:${++bridgeSequence}`,
      nextOperationRequestId: () => fixture.snapshotQuery.requestId,
    }));
    expect(await client.snapshot()).toEqual(fixture.snapshotResponse);
    expect(await client.mutate(fixture.mutationCommands[0]!)).toEqual(fixture.mutationResults[0]);
    expect(await client.cancel(fixture.cancellationCommand)).toEqual(fixture.cancellationResult);
  });

  test("rejects wrong bridge correlation and wrong operation identity", () => {
    const correlation = new BridgeOperationJobCorrelation({
      bridgeRequestId: "bridge-request:1",
      bridgeJobId: "bridge-job:1",
      operationId: "operation:scan",
    });
    expect(correlation.acceptProgress({
      requestId: "bridge-request:other",
      jobId: "bridge-job:1",
      progress: fixture.changedEvents[0]!,
    })).toBeUndefined();
    const foreign = structuredClone(fixture.changedEvents[0]!);
    foreign.operationId = "operation:foreign";
    expect(correlation.acceptProgress({
      requestId: "bridge-request:1",
      jobId: "bridge-job:1",
      progress: foreign,
    })).toBeUndefined();
  });
});

function operationSession(): BridgeSession {
  return new BridgeSession({
    protocolVersion: 1,
    host: { hostInstanceId: "host:operation", form: "localService" },
    sessionId: "session:operation",
    connection: { state: "ready", reason: "negotiationAccepted" },
    authentication: "authenticated",
    transportFeatures: [],
    domainCapabilities: [{
      domainId: "longhorn.operation",
      capabilities: ["longhorn.operation.read", "longhorn.operation.manage", "longhorn.operation.cancel"],
    }],
    domainAuthorities: [{
      domainId: "longhorn.operation",
      scopeId: "scope:operation",
      availability: "available",
      readAuthority: "authoritative",
      writeAuthority: "authoritative",
      executionAuthority: "executor",
      authorityEpoch: 7,
      authoritativeRevision: 0,
    }],
    diagnostics: [],
  });
}
