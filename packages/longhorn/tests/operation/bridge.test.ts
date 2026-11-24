import assert from "node:assert/strict";
import { describe, expect, test } from "bun:test";
import { BridgeDomainClient, BridgeSession } from "@inflatable-cookie/longhorn/bridge";
import type { BridgeFailure, BridgeOperationDescriptor } from "@inflatable-cookie/longhorn/bridge";

import {
  BridgeOperationError,
  BridgeOperationJobCorrelation,
  createBridgeOperationPort,
} from "../../src/operation/bridge.ts";
import { OperationClient } from "../../src/operation/client.ts";
import { fixture } from "./support.ts";

const failure: BridgeFailure<null> = {
  code: "operation:unavailable",
  message: "The operation authority could not answer",
  retryClass: "afterReconnect",
  phase: "transport",
  details: null,
};

describe("bridge operation composition", () => {
  test("converges with the operation trace while bridge ids remain correlation only", async () => {
    const session = operationSession();
    const adapter = {
      async invoke(operation: BridgeOperationDescriptor<unknown, unknown>, request: unknown) {
        const envelope = request as {
          context: { requestId: string };
          payload: { requestId: string };
        };
        if (operation.route.endsWith("snapshot")) {
          return {
            requestId: envelope.context.requestId,
            outcome: { success: fixture.snapshotResponse },
          };
        }
        if (operation.route.endsWith("mutate")) {
          const result = fixture.mutationResults.find(
            (candidate) => candidate.requestId === envelope.payload.requestId,
          );
          return {
            requestId: envelope.context.requestId,
            authoritativeRevision: result?.snapshot.catalogueRevision ?? null,
            outcome: { applied: result },
          };
        }
        return {
          requestId: envelope.context.requestId,
          outcome: { success: fixture.cancellationResult },
        };
      },
    };
    let bridgeSequence = 0;
    const client = new OperationClient(
      createBridgeOperationPort({
        client: new BridgeDomainClient(session, adapter),
        domainId: "longhorn.operation",
        sessionId: "session:operation",
        authorityEpoch: 7,
        nextBridgeRequestId: () => `bridge-request:${++bridgeSequence}`,
        nextOperationRequestId: () => fixture.snapshotQuery.requestId,
      }),
    );
    expect(await client.snapshot()).toEqual(fixture.snapshotResponse);
    const [command] = fixture.mutationCommands;
    const [expected] = fixture.mutationResults;
    assert(command && expected);
    expect(await client.mutate(command)).toEqual(expected);
    expect(await client.cancel(fixture.cancellationCommand)).toEqual(fixture.cancellationResult);
  });

  test("rejects wrong bridge correlation and wrong operation identity", () => {
    const correlation = new BridgeOperationJobCorrelation({
      bridgeRequestId: "bridge-request:1",
      bridgeJobId: "bridge-job:1",
      operationId: "operation:scan",
    });
    expect(
      correlation.acceptProgress({
        requestId: "bridge-request:other",
        jobId: "bridge-job:1",
        progress: fixture.changedEvents[0]!,
      }),
    ).toBeUndefined();
    const foreign = structuredClone(fixture.changedEvents[0]!);
    foreign.operationId = "operation:foreign";
    expect(
      correlation.acceptProgress({
        requestId: "bridge-request:1",
        jobId: "bridge-job:1",
        progress: foreign,
      }),
    ).toBeUndefined();
  });

  test("query and cancellation rejections preserve the bridge failure", async () => {
    const requestId = "bridge-request:1";
    const client = clientReturning({
      requestId,
      outcome: { rejected: failure },
    });
    const expected = new BridgeOperationError({ kind: "rejected", requestId, failure });
    await expect(client.snapshot()).rejects.toEqual(expected);
    await expect(client.cancel(fixture.cancellationCommand)).rejects.toEqual(expected);
  });

  test("mutation rejection, indeterminate delivery and wrong correlation remain distinct", async () => {
    const [command] = fixture.mutationCommands;
    assert(command);
    const requestId = "bridge-request:1";
    for (const kind of ["rejected", "indeterminate"] as const) {
      const client = clientReturning({
        requestId,
        authoritativeRevision: null,
        outcome: { [kind]: failure },
      });
      await expect(client.mutate(command)).rejects.toBeInstanceOf(BridgeOperationError);
      await expect(client.mutate(command)).rejects.toMatchObject({
        outcome: { kind, requestId, failure },
      });
    }

    const receivedRequestId = "bridge-request:other";
    const foreign = clientReturning({
      requestId: receivedRequestId,
      authoritativeRevision: null,
      outcome: { indeterminate: failure },
    });
    await expect(foreign.mutate(command)).rejects.toMatchObject({
      outcome: { kind: "miscorrelated", requestId, receivedRequestId },
    });
  });

  test("failed and cancelled terminals settle the job with typed errors", () => {
    const requestId = "bridge-request:1";
    const jobId = "bridge-job:1";
    for (const outcome of [{ failed: failure }, "cancelled"] as const) {
      const correlation = new BridgeOperationJobCorrelation({
        bridgeRequestId: requestId,
        bridgeJobId: jobId,
        operationId: "operation:scan",
      });
      const event = { requestId, jobId, outcome };
      expect(correlation.acceptTerminal({ ...event, jobId: "bridge-job:other" })).toBeUndefined();
      let caught: unknown;
      try {
        correlation.acceptTerminal(event);
      } catch (error) {
        caught = error;
      }
      assert(caught instanceof BridgeOperationError);
      expect(caught.outcome).toEqual(
        outcome === "cancelled"
          ? { kind: "cancelled", requestId, jobId }
          : { kind: "failed", requestId, jobId, failure },
      );
      expect(correlation.acceptTerminal(event)).toBeUndefined();
    }
  });
});

function clientReturning(reply: unknown): OperationClient {
  return new OperationClient(
    createBridgeOperationPort({
      client: new BridgeDomainClient(operationSession(), { invoke: async () => reply }),
      domainId: "longhorn.operation",
      sessionId: "session:operation",
      authorityEpoch: 7,
      nextBridgeRequestId: () => "bridge-request:1",
      nextOperationRequestId: () => fixture.snapshotQuery.requestId,
    }),
  );
}

function operationSession(): BridgeSession {
  return new BridgeSession({
    protocolVersion: 1,
    host: { hostInstanceId: "host:operation", form: "localService" },
    sessionId: "session:operation",
    connection: { state: "ready", reason: "negotiationAccepted" },
    authentication: "authenticated",
    transportFeatures: [],
    domainCapabilities: [
      {
        domainId: "longhorn.operation",
        capabilities: [
          "longhorn.operation.read",
          "longhorn.operation.manage",
          "longhorn.operation.cancel",
        ],
      },
    ],
    domainAuthorities: [
      {
        domainId: "longhorn.operation",
        scopeId: "scope:operation",
        availability: "available",
        readAuthority: "authoritative",
        writeAuthority: "authoritative",
        executionAuthority: "executor",
        authorityEpoch: 7,
        authoritativeRevision: 0,
      },
    ],
    diagnostics: [],
  });
}
