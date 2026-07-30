import { readFileSync } from "node:fs";

import {
  BridgeDomainClient,
  BridgeHostRouter,
  BridgeSession,
  BridgeSessionClient,
  DirectBridgeAdapter,
  SerializedLoopbackBridgeAdapter,
  bridgeCodec,
  parseBridgeCancellationReceipt,
  parseBridgeCancellationRequest,
  parseBridgeHelloRequest,
  parseBridgeNegotiationReceipt,
  parseBridgeQueryEnvelope,
  parseBridgeQueryReply,
  record,
  type BridgeCancellationReceipt,
  type BridgeCancellationRequest,
  type BridgeCodec,
  type BridgeHelloRequest,
  type BridgeHostForm,
  type BridgeNegotiationReceipt,
  type BridgeOperationAdapter,
  type BridgeOperationDescriptor,
  type BridgeQueryEnvelope,
  type BridgeQueryReply,
  type BridgeSessionId,
  type DomainAuthorityDescriptor,
  type DomainId,
} from "@longhorn/bridge";
import {
  BRIDGE_CANCEL_COMMAND,
  BRIDGE_HELLO_COMMAND,
  BRIDGE_QUERY_COMMAND,
  connectTauriBridge,
} from "@longhorn/bridge/tauri";

export type ShapeName =
  | "bovine"
  | "jetstream"
  | "soundcheck"
  | "nucleus"
  | "loophole";

export interface DomainDeclaration {
  readonly domainId: DomainId;
  readonly capabilities: readonly string[];
  readonly authority: Omit<DomainAuthorityDescriptor, "domainId">;
}

export interface ShapeDeclaration {
  readonly hostForms: readonly BridgeHostForm[];
  readonly transportFeatures: readonly string[];
  readonly imports: readonly string[];
  readonly tauriPermissions: readonly string[];
  readonly serviceOwnership:
    | "ownedLocal"
    | "externalLocal"
    | "externalRemote"
    | null;
  readonly maximumQueryRetries: number;
  readonly domains: readonly DomainDeclaration[];
}

export interface TopologyDeclarations {
  readonly schema: "longhorn.bridge-topology-declarations.v1";
  readonly shapes: Readonly<Record<ShapeName, ShapeDeclaration>>;
}

interface ProbePayload {
  readonly value: number;
}

interface ProbeFailure {
  readonly code: string;
}

type QueryRequest = BridgeQueryEnvelope<ProbePayload>;
type QueryReply = BridgeQueryReply<ProbePayload, ProbeFailure>;

export type AdapterName = "direct" | "tauri" | "loopback";

export const adapterNames = ["direct", "tauri", "loopback"] as const;

export const declarations = JSON.parse(
  readFileSync(new URL("declarations.json", import.meta.url), "utf8"),
) as TopologyDeclarations;

const probeCodec = bridgeCodec<ProbePayload>((value) => {
  const source = record(value, ["value"]);
  if (!Number.isSafeInteger(source.value)) {
    throw new TypeError("probe value must be an integer");
  }
  return { value: source.value as number };
});

const failureCodec = bridgeCodec<ProbeFailure>((value) => {
  const source = record(value, ["code"]);
  if (typeof source.code !== "string") {
    throw new TypeError("probe failure code must be a string");
  }
  return { code: source.code };
});

const cancellationReplyCodec = bridgeCodec<
  BridgeCancellationReceipt<ProbeFailure>
>((value) =>
  parseBridgeCancellationReceipt(value, failureCodec)
);

export function declaration(name: ShapeName): ShapeDeclaration {
  return declarations.shapes[name];
}

export function hello(
  name: ShapeName,
  domains = declaration(name).domains.map(({ domainId }) => domainId),
): BridgeHelloRequest {
  return parseBridgeHelloRequest({
    protocolVersion: 1,
    bridgeId: `bridge:fixture-${name}`,
    requestedDomains: domains,
  });
}

export function receipt(
  name: ShapeName,
  form = declaration(name).hostForms[0]!,
  sessionId: BridgeSessionId = `session:fixture-${name}`,
  domains = declaration(name).domains,
): BridgeNegotiationReceipt {
  return parseBridgeNegotiationReceipt(
    {
      protocolVersion: 1,
      host: {
        hostInstanceId: `host:fixture-${name}-${form.toLowerCase()}`,
        form,
      },
      sessionId,
      connection: {
        state: "ready",
        reason: "negotiationAccepted",
      },
      authentication: form === "remote" ? "authenticated" : "notRequired",
      transportFeatures: [...declaration(name).transportFeatures],
      domainCapabilities: domains.map((domain) => ({
        domainId: domain.domainId,
        capabilities: [...domain.capabilities],
      })),
      domainAuthorities: domains.map((domain) => ({
        domainId: domain.domainId,
        ...domain.authority,
      })),
      diagnostics: [],
    },
    hello(name, domains.map(({ domainId }) => domainId)),
  );
}

export function session(
  name: ShapeName,
  form?: BridgeHostForm,
  domains?: readonly DomainDeclaration[],
): BridgeSession {
  return new BridgeSession(receipt(name, form, undefined, domains));
}

export function readableDomain(name: ShapeName): DomainDeclaration {
  const domain = declaration(name).domains.find(
    ({ authority }) =>
      authority.availability !== "offline" &&
      authority.readAuthority !== "none",
  );
  if (domain === undefined) {
    throw new Error(`${name} has no readable fixture domain`);
  }
  return domain;
}

export async function queryParity(
  name: ShapeName,
  selectedDomain?: DomainDeclaration,
): Promise<{
  readonly domainId: DomainId;
  readonly traces: Readonly<Record<AdapterName, QueryReply>>;
}> {
  const domain = selectedDomain ?? readableDomain(name);
  const negotiated = receipt(name);
  const operation = queryOperation(domain.domainId);
  const request = queryRequest(
    negotiated.sessionId,
    domain.domainId,
    `request:fixture-${name}`,
  );
  const router = routerFor(
    negotiated,
    operation,
    (candidate) => ({
      requestId: candidate.context.requestId,
      outcome: { success: { value: candidate.payload.value + 1 } },
    }),
  );

  const direct = new DirectBridgeAdapter(router);
  const loopback = new SerializedLoopbackBridgeAdapter(router);
  const tauri = new RouterInvokeTransport(router);
  const traces = {
    direct: await execute(hello(name), direct, operation, request),
    tauri: await executeTauri(hello(name), tauri, operation, request),
    loopback: await execute(hello(name), loopback, operation, request),
  };
  return {
    domainId: domain.domainId,
    traces,
  };
}

export async function cancellationParity(
  name: ShapeName,
  domain: DomainDeclaration,
): Promise<Readonly<Record<AdapterName, BridgeCancellationReceipt<ProbeFailure>>>> {
  const activeDomain: DomainDeclaration = {
    ...domain,
    authority: {
      ...domain.authority,
      availability: "available",
      executionAuthority: "executor",
    },
  };
  const negotiated = receipt(
    name,
    declaration(name).hostForms.at(-1),
    `session:fixture-${name}-job`,
    [activeDomain],
  );
  const operation = cancellationOperation(activeDomain.domainId);
  const request: BridgeCancellationRequest = {
    context: {
      requestId: `request:fixture-${name}-cancel`,
      sessionId: negotiated.sessionId,
      domainId: activeDomain.domainId,
    },
    targetRequestId: `request:fixture-${name}-job`,
    jobId: `job:fixture-${name}`,
  };
  const router = routerFor(
    negotiated,
    operation,
    (candidate) => ({
      requestId: candidate.context.requestId,
      targetRequestId: candidate.targetRequestId,
      jobId: candidate.jobId,
      outcome: "accepted" as const,
    }),
  );
  const direct = new DirectBridgeAdapter(router);
  const loopback = new SerializedLoopbackBridgeAdapter(router);
  const tauri = new RouterInvokeTransport(router);
  const requested = hello(name, [activeDomain.domainId]);
  return {
    direct: await execute(requested, direct, operation, request),
    tauri: await executeTauri(requested, tauri, operation, request),
    loopback: await execute(requested, loopback, operation, request),
  };
}

export function sameValues<T>(
  values: Readonly<Record<AdapterName, T>>,
): boolean {
  const encoded = adapterNames.map((name) => JSON.stringify(values[name]));
  return new Set(encoded).size === 1;
}

export function authorityProjection(
  negotiated: BridgeNegotiationReceipt,
): readonly DomainAuthorityDescriptor[] {
  return negotiated.domainAuthorities.map((authority) => ({
    ...authority,
  }));
}

export function jsonRoundTrip<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export const unknownCodec: BridgeCodec<unknown> = bridgeCodec(
  (value) => value,
);

function queryOperation(
  domainId: DomainId,
): BridgeOperationDescriptor<QueryRequest, QueryReply> {
  return {
    route: `${domainId}.fixture_query`,
    domainId,
    kind: "query",
    requiredCapability: "query",
    request: bridgeCodec((value) =>
      parseBridgeQueryEnvelope(value, probeCodec)
    ),
    reply: bridgeCodec((value) =>
      parseBridgeQueryReply(value, probeCodec, failureCodec)
    ),
  };
}

function cancellationOperation(
  domainId: DomainId,
): BridgeOperationDescriptor<
  BridgeCancellationRequest,
  BridgeCancellationReceipt<ProbeFailure>
> {
  return {
    route: `${domainId}.fixture_cancel`,
    domainId,
    kind: "cancellation",
    requiredCapability: "cancel_job",
    request: bridgeCodec(parseBridgeCancellationRequest),
    reply: cancellationReplyCodec,
  };
}

function queryRequest(
  sessionId: BridgeSessionId,
  domainId: DomainId,
  requestId: string,
): QueryRequest {
  return {
    context: { requestId, sessionId, domainId },
    payload: { value: 7 },
  };
}

function routerFor<Request, Reply>(
  negotiated: BridgeNegotiationReceipt,
  operation: BridgeOperationDescriptor<Request, Reply>,
  handler: (request: Request) => Reply,
): BridgeHostRouter {
  const router = new BridgeHostRouter(() => negotiated);
  router.register(operation, handler);
  return router;
}

async function execute<Request, Reply>(
  request: BridgeHelloRequest,
  adapter: DirectBridgeAdapter | SerializedLoopbackBridgeAdapter,
  operation: BridgeOperationDescriptor<Request, Reply>,
  value: Request,
): Promise<Reply> {
  const connected = await new BridgeSessionClient(adapter).connect(request);
  return new BridgeDomainClient(connected, adapter).execute(operation, value);
}

async function executeTauri<Request, Reply>(
  request: BridgeHelloRequest,
  transport: RouterInvokeTransport,
  operation: BridgeOperationDescriptor<Request, Reply>,
  value: Request,
): Promise<Reply> {
  const connected = await connectTauriBridge(request, transport);
  return connected.domain.execute(operation, value);
}

class RouterInvokeTransport {
  readonly #router: BridgeHostRouter;

  constructor(router: BridgeHostRouter) {
    this.#router = router;
  }

  invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    if (command === BRIDGE_HELLO_COMMAND) {
      return this.#router.negotiate(arguments_.request);
    }
    if (
      command === BRIDGE_QUERY_COMMAND ||
      command === BRIDGE_CANCEL_COMMAND
    ) {
      return this.#router.invoke(
        String(arguments_.route),
        arguments_.request,
      );
    }
    throw new Error(`unexpected fixture Tauri command: ${command}`);
  }
}
