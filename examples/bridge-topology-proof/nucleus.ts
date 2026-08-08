import {
  BridgeAuthorityError,
  BridgeDomainClient,
  bridgeCodec,
  parseBridgeCancellationRequest,
  parseBridgeCommandEnvelope,
  record,
  type BridgeCancellationRequest,
  type BridgeCommandEnvelope,
  type BridgeOperationDescriptor,
} from "@inflatable-cookie/longhorn/bridge";
import { BridgeServiceRuntime } from "@inflatable-cookie/longhorn/bridge/supervision";

import {
  cancellationParity,
  declaration,
  queryParity,
  receipt,
  sameValues,
  session,
  unknownCodec,
} from "./common.ts";

type FixtureCommand = BridgeCommandEnvelope<Record<string, never>>;

const emptyCodec = bridgeCodec<Record<string, never>>((value) => {
  record(value, []);
  return {};
});

export async function runNucleusTrace() {
  const fixture = declaration("nucleus");
  const local = fixture.domains[0]!;
  const executor = fixture.domains[1]!;
  const embedded = receipt("nucleus", "direct");
  const optionalHost = receipt(
    "nucleus",
    "localService",
    "session:fixture-nucleus-service",
  );
  const optionalService = new BridgeServiceRuntime("externalLocal");
  const attach = await optionalService.execute(
    { perform: () => "accepted" },
    "attach",
  );
  const current = session("nucleus");
  const inertAdapter = {
    invoke: () => Promise.reject(new Error("authority should reject first")),
  };
  const client = new BridgeDomainClient(current, inertAdapter);

  const writeDenied = await authorityError(
    client.execute(command(executor.domainId), {
      context: context(executor.domainId),
      authorityEpoch: executor.authority.authorityEpoch,
      expectedRevision: null,
      idempotencyKey: null,
      payload: {},
    }),
  );
  const executionDenied = await authorityError(
    client.execute(cancellation(local.domainId), {
      context: context(local.domainId),
      targetRequestId: "request:fixture-target",
      jobId: "job:fixture-nucleus",
    }),
  );
  const execution = await cancellationParity("nucleus", executor);
  const query = await queryParity("nucleus");

  return {
    hostConnectionSeparate: embedded.host.form !== optionalHost.host.form,
    optionalHostLifecycle: attach.current,
    domainIdentityStable: embedded.domainAuthorities.map((value) =>
      value.domainId
    ).join(",") === optionalHost.domainAuthorities.map((value) =>
      value.domainId
    ).join(","),
    capabilityDoesNotGrantWrite: writeDenied,
    capabilityDoesNotGrantExecution: executionDenied,
    executionParity: sameValues(execution),
    queryParity: sameValues(query.traces),
    authority: {
      [local.domainId]: local.authority,
      [executor.domainId]: executor.authority,
    },
  } as const;
}

function context(domainId: string) {
  return {
    requestId: "request:fixture-nucleus-authority",
    sessionId: "session:fixture-nucleus",
    domainId,
  };
}

function command(
  domainId: string,
): BridgeOperationDescriptor<FixtureCommand, unknown> {
  return {
    route: `${domainId}.fixture_command`,
    domainId,
    kind: "command",
    requiredCapability: "mutate",
    request: bridgeCodec((value) =>
      parseBridgeCommandEnvelope(value, emptyCodec)
    ),
    reply: unknownCodec,
  };
}

function cancellation(
  domainId: string,
): BridgeOperationDescriptor<BridgeCancellationRequest, unknown> {
  return {
    route: `${domainId}.fixture_cancel`,
    domainId,
    kind: "cancellation",
    requiredCapability: "cancel_job",
    request: bridgeCodec(parseBridgeCancellationRequest),
    reply: unknownCodec,
  };
}

async function authorityError(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
  } catch (error) {
    if (error instanceof BridgeAuthorityError) {
      return error.code;
    }
    throw error;
  }
  throw new Error("expected authority rejection");
}
