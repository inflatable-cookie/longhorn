import type {
  BridgeCommandDelivery,
  BridgeCommandEnvelope,
  BridgeCommandRetryDecision,
  BridgeDeduplicationSupport,
  BridgeQueryEnvelope,
  BridgeQueryRetryDecision,
  BridgeRetryClass,
  DomainId,
} from "./generated/protocol.ts";
import type { BridgeCodec } from "./validation.ts";
import { BridgeSession } from "./session.ts";

export type BridgeOperationKind = "query" | "command" | "cancellation";

export interface BridgeOperationDescriptor<Request, Reply> {
  readonly route: string;
  readonly domainId: DomainId;
  readonly kind: BridgeOperationKind;
  readonly requiredCapability: string;
  readonly request: BridgeCodec<Request>;
  readonly reply: BridgeCodec<Reply>;
}

export interface BridgeOperationAdapter {
  invoke<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): Promise<unknown>;
}

export class BridgeDomainClient {
  readonly #session: BridgeSession;
  readonly #adapter: BridgeOperationAdapter;

  constructor(session: BridgeSession, adapter: BridgeOperationAdapter) {
    this.#session = session;
    this.#adapter = adapter;
  }

  async execute<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): Promise<Reply> {
    const checkedRequest = operation.request.parse(request);
    this.#assertAuthority(operation, checkedRequest);
    return operation.reply.parse(
      await this.#adapter.invoke(operation, checkedRequest),
    );
  }

  #assertAuthority<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): void {
    if (
      !this.#session.supports(
        operation.domainId,
        operation.requiredCapability,
      )
    ) {
      throw new BridgeAuthorityError(
        "capability_unavailable",
        operation.domainId,
      );
    }
    const authority = this.#session.authority(operation.domainId);
    const context = (
      request as {
        readonly context?: {
          readonly domainId?: unknown;
          readonly sessionId?: unknown;
        };
      }
    ).context;
    if (context?.domainId !== operation.domainId) {
      throw new BridgeAuthorityError(
        "request_domain_mismatch",
        operation.domainId,
      );
    }
    if (context.sessionId !== this.#session.receipt.sessionId) {
      throw new BridgeAuthorityError(
        "request_session_mismatch",
        operation.domainId,
      );
    }
    if (authority === undefined || authority.availability === "offline") {
      throw new BridgeAuthorityError(
        "authority_unavailable",
        operation.domainId,
      );
    }
    if (operation.kind === "query" && authority.readAuthority === "none") {
      throw new BridgeAuthorityError("read_denied", operation.domainId);
    }
    if (operation.kind === "command") {
      if (authority.writeAuthority !== "authoritative") {
        throw new BridgeAuthorityError("write_denied", operation.domainId);
      }
      const envelope = request as BridgeCommandEnvelope<unknown>;
      if (envelope.authorityEpoch !== authority.authorityEpoch) {
        throw new BridgeAuthorityError("stale_authority", operation.domainId);
      }
    }
    if (
      operation.kind === "cancellation" &&
      authority.executionAuthority !== "executor"
    ) {
      throw new BridgeAuthorityError("execution_denied", operation.domainId);
    }
  }
}

export type BridgeAuthorityErrorCode =
  | "capability_unavailable"
  | "authority_unavailable"
  | "read_denied"
  | "write_denied"
  | "execution_denied"
  | "request_domain_mismatch"
  | "request_session_mismatch"
  | "stale_authority";

export class BridgeAuthorityError extends Error {
  readonly code: BridgeAuthorityErrorCode;
  readonly domainId: DomainId;

  constructor(code: BridgeAuthorityErrorCode, domainId: DomainId) {
    super(`bridge authority check failed: ${code} (${domainId})`);
    this.name = "BridgeAuthorityError";
    this.code = code;
    this.domainId = domainId;
  }
}

export function classifyBridgeCommandRetry<P>(
  command: BridgeCommandEnvelope<P>,
  delivery: BridgeCommandDelivery,
  retryClass: BridgeRetryClass,
  deduplication: BridgeDeduplicationSupport,
): BridgeCommandRetryDecision {
  if (delivery === "notDispatched") {
    return "doNotRetry";
  }
  if (
    retryClass !== "never" &&
    command.idempotencyKey !== null &&
    typeof deduplication === "object" &&
    deduplication !== null &&
    "finite" in deduplication
  ) {
    return "retrySameRequest";
  }
  return "indeterminate";
}

export function classifyBridgeQueryRetry<P>(
  _query: BridgeQueryEnvelope<P>,
  retryClass: BridgeRetryClass,
  adapterAllowsRetry: boolean,
): BridgeQueryRetryDecision {
  return adapterAllowsRetry && retryClass !== "never"
    ? "retry"
    : "doNotRetry";
}
