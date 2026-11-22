import type {
  BridgeAuthorityCursorDecision,
  BridgeAuthorityRequirement,
  BridgeConnectionReason,
  BridgeConnectionState,
  BridgeConnectionStatus,
  BridgeConnectionTransitionReceipt,
  BridgeHelloRequest,
  BridgeNegotiationReceipt,
  BridgeReconnectSchedule,
  BridgeRequiredAuthority,
  BridgeRetryClass,
  BridgeStreamCursor,
  DomainAuthorityDescriptor,
} from "../generated/protocol.ts";
import {
  parseBridgeHelloRequest,
  parseBridgeNegotiationReceipt,
  parseBridgeStreamCursor,
} from "../validation.ts";
import { BridgeSession } from "../session.ts";
import {
  checkedDeadline,
  checkedIncrement,
  checkedMonotonic,
  checkedRetryClass,
  checkedRetryLimit,
  BridgeRuntimeError,
  type BridgeRuntimeBackoff,
  type BridgeRuntimeClock,
} from "./policy.ts";

export class BridgeConnectionRuntime {
  readonly #clock: BridgeRuntimeClock;
  readonly #backoff: BridgeRuntimeBackoff;
  readonly #reconnectLimit: number;
  #status: BridgeConnectionStatus = { state: "idle", reason: null };
  #sequence = 0;
  #reconnectAttempts = 0;
  #reconnectNotBefore: number | undefined;
  #session: BridgeSession | undefined;

  constructor(
    clock: BridgeRuntimeClock,
    backoff: BridgeRuntimeBackoff,
    reconnectLimit: number,
  ) {
    this.#clock = clock;
    this.#backoff = backoff;
    this.#reconnectLimit = checkedRetryLimit(reconnectLimit);
  }

  get status(): BridgeConnectionStatus {
    return this.#status;
  }

  get session(): BridgeSession | undefined {
    return this.#session;
  }

  connect(): BridgeConnectionTransitionReceipt {
    return this.#transition("connecting", "connectRequested");
  }

  transportReady(): BridgeConnectionTransitionReceipt {
    const at = checkedMonotonic(this.#clock.now());
    if (
      this.#status.state === "reconnecting" &&
      this.#reconnectNotBefore !== undefined &&
      at < this.#reconnectNotBefore
    ) {
      throw new BridgeRuntimeError(
        "retry_not_due",
        "reconnect backoff has not elapsed",
      );
    }
    this.#requireTransition("negotiating");
    const receipt = this.#prepareAt(
      "negotiating",
      "transportReady",
      at,
      null,
      this.#session,
    );
    this.#reconnectNotBefore = undefined;
    return this.#apply(receipt);
  }

  acceptNegotiation(
    value: unknown,
    requirements: readonly BridgeAuthorityRequirement[],
    request?: BridgeHelloRequest,
  ): BridgeConnectionTransitionReceipt {
    this.#requireTransition("ready");
    const checkedRequest = request === undefined
      ? undefined
      : parseBridgeHelloRequest(request);
    const receipt = parseBridgeNegotiationReceipt(value, checkedRequest);
    assertRequiredAuthorities(receipt, requirements);
    const session = new BridgeSession(receipt);
    const transition = this.#prepareAt(
      "ready",
      "negotiationAccepted",
      checkedMonotonic(this.#clock.now()),
      null,
      session,
    );
    this.#session = session;
    this.#reconnectAttempts = 0;
    this.#reconnectNotBefore = undefined;
    return this.#apply(transition);
  }

  degrade(
    reason: Extract<
      BridgeConnectionReason,
      "capabilityChanged" | "transportLost" | "hostFailure"
    >,
  ): BridgeConnectionTransitionReceipt {
    if (
      reason !== "capabilityChanged" &&
      reason !== "transportLost" &&
      reason !== "hostFailure"
    ) {
      throw new BridgeRuntimeError(
        "invalid_transition",
        "invalid degraded-state reason",
      );
    }
    return this.#transition("degraded", reason);
  }

  reconnect(retryClass: BridgeRetryClass): BridgeConnectionTransitionReceipt {
    retryClass = checkedRetryClass(retryClass);
    const attempt = this.#reconnectAttempts + 1;
    const admitted = retryClass !== "never" &&
      attempt <= this.#reconnectLimit;
    this.#requireTransition(admitted ? "reconnecting" : "offline");
    const at = checkedMonotonic(this.#clock.now());
    const reconnect: BridgeReconnectSchedule | null = admitted
      ? {
        attempt,
        retryClass,
        notBefore: checkedDeadline(at, this.#backoff.delay(retryClass, attempt)),
      }
      : null;
    let reason: BridgeConnectionReason = "transportLost";
    if (admitted) reason = "retryScheduled";
    else if (retryClass === "never") reason = "retryForbidden";
    const receipt = this.#prepareAt(
      admitted ? "reconnecting" : "offline",
      reason,
      at,
      reconnect,
      undefined,
    );
    if (reconnect !== null) {
      this.#reconnectAttempts = reconnect.attempt;
    }
    this.#reconnectNotBefore = reconnect?.notBefore;
    this.#session = undefined;
    return this.#apply(receipt);
  }

  incompatible(): BridgeConnectionTransitionReceipt {
    return this.#terminal("incompatible", "versionMismatch");
  }

  unauthorized(): BridgeConnectionTransitionReceipt {
    return this.#terminal("unauthorized", "authorizationRejected");
  }

  fail(): BridgeConnectionTransitionReceipt {
    return this.#terminal("failed", "hostFailure");
  }

  close(): BridgeConnectionTransitionReceipt {
    return this.#terminal("closed", "shutdown");
  }

  classifyCursor(cursor: BridgeStreamCursor): BridgeAuthorityCursorDecision {
    cursor = parseBridgeStreamCursor(cursor);
    const session = this.#session;
    if (session === undefined || cursor.sessionId !== session.receipt.sessionId) {
      return "supersededSession";
    }
    const authority = session.authority(cursor.domainId);
    if (authority === undefined) {
      return "unknownDomain";
    }
    if (cursor.authorityEpoch < authority.authorityEpoch) {
      return "staleAuthority";
    }
    if (cursor.authorityEpoch > authority.authorityEpoch) {
      return "refreshAuthority";
    }
    return "current";
  }

  #terminal(
    state: Extract<
      BridgeConnectionState,
      "incompatible" | "unauthorized" | "failed" | "closed"
    >,
    reason: BridgeConnectionReason,
  ): BridgeConnectionTransitionReceipt {
    this.#requireTransition(state);
    const receipt = this.#prepareAt(
      state, reason, checkedMonotonic(this.#clock.now()), null, undefined,
    );
    this.#session = undefined;
    this.#reconnectNotBefore = undefined;
    return this.#apply(receipt);
  }

  #transition(
    state: BridgeConnectionState,
    reason: BridgeConnectionReason,
  ): BridgeConnectionTransitionReceipt {
    this.#requireTransition(state);
    return this.#apply(this.#prepareAt(
      state, reason, checkedMonotonic(this.#clock.now()), null, this.#session,
    ));
  }

  #prepareAt(
    state: BridgeConnectionState,
    reason: BridgeConnectionReason,
    at: number,
    reconnect: BridgeReconnectSchedule | null,
    session: BridgeSession | undefined,
  ): BridgeConnectionTransitionReceipt {
    return {
      sequence: checkedIncrement(this.#sequence),
      at,
      previous: this.#status,
      current: { state, reason },
      sessionId: session?.receipt.sessionId ?? null,
      reconnect,
    };
  }

  #apply(receipt: BridgeConnectionTransitionReceipt): BridgeConnectionTransitionReceipt {
    this.#sequence = receipt.sequence;
    this.#status = receipt.current;
    return receipt;
  }

  #requireTransition(next: BridgeConnectionState): void {
    if (!transitionAllowed(this.#status.state, next)) {
      throw new BridgeRuntimeError(
        "invalid_transition",
        `cannot transition from ${this.#status.state} to ${next}`,
      );
    }
  }
}

function assertRequiredAuthorities(
  receipt: BridgeNegotiationReceipt,
  requirements: readonly BridgeAuthorityRequirement[],
): void {
  for (const requirement of requirements) {
    const authority = receipt.domainAuthorities.find(
      (candidate) => candidate.domainId === requirement.domainId,
    );
    if (
      authority === undefined ||
      authority.availability === "offline" ||
      !authoritySatisfies(authority, requirement.authority)
    ) {
      throw new BridgeRuntimeError(
        "required_authority_unavailable",
        `${requirement.authority} unavailable for ${requirement.domainId}`,
      );
    }
  }
}

function authoritySatisfies(
  authority: DomainAuthorityDescriptor,
  required: BridgeRequiredAuthority,
): boolean {
  switch (required) {
    case "available":
      return true;
    case "readable":
      return authority.readAuthority !== "none";
    case "authoritativeRead":
      return authority.readAuthority === "authoritative";
    case "writable":
      return authority.writeAuthority === "authoritative";
    case "executable":
      return authority.executionAuthority === "executor";
  }
}

function transitionAllowed(
  current: BridgeConnectionState,
  next: BridgeConnectionState,
): boolean {
  if (next === "closed") {
    return current !== "closed";
  }
  const transitions: Partial<
    Record<BridgeConnectionState, readonly BridgeConnectionState[]>
  > = {
    idle: ["connecting"],
    connecting: ["negotiating", "reconnecting", "offline", "failed"],
    negotiating: [
      "ready",
      "reconnecting",
      "offline",
      "incompatible",
      "unauthorized",
      "failed",
    ],
    ready: ["degraded", "reconnecting", "offline", "failed"],
    degraded: ["ready", "reconnecting", "offline", "failed"],
    reconnecting: ["negotiating", "reconnecting", "offline", "failed"],
    offline: ["reconnecting", "failed"],
  };
  return transitions[current]?.includes(next) ?? false;
}
