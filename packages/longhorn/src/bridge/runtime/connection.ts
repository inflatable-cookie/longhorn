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
    const receipt = this.#commitAt(
      "negotiating",
      "transportReady",
      at,
      null,
    );
    this.#reconnectNotBefore = undefined;
    return receipt;
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
    this.#session = new BridgeSession(receipt);
    this.#reconnectAttempts = 0;
    this.#reconnectNotBefore = undefined;
    return this.#commit("ready", "negotiationAccepted", null);
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
    let reconnect: BridgeReconnectSchedule | null = null;
    if (admitted) {
      reconnect = {
        attempt,
        retryClass,
        notBefore: checkedDeadline(
          at,
          this.#backoff.delay(retryClass, attempt),
        ),
      };
      this.#reconnectAttempts = attempt;
      this.#reconnectNotBefore = reconnect.notBefore;
    } else {
      this.#reconnectNotBefore = undefined;
    }
    this.#session = undefined;
    return this.#commitAt(
      admitted ? "reconnecting" : "offline",
      admitted ? "retryScheduled" : "transportLost",
      at,
      reconnect,
    );
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
    this.#session = undefined;
    this.#reconnectNotBefore = undefined;
    return this.#commit(state, reason, null);
  }

  #transition(
    state: BridgeConnectionState,
    reason: BridgeConnectionReason,
  ): BridgeConnectionTransitionReceipt {
    this.#requireTransition(state);
    return this.#commit(state, reason, null);
  }

  #commit(
    state: BridgeConnectionState,
    reason: BridgeConnectionReason,
    reconnect: BridgeReconnectSchedule | null,
  ): BridgeConnectionTransitionReceipt {
    return this.#commitAt(
      state,
      reason,
      checkedMonotonic(this.#clock.now()),
      reconnect,
    );
  }

  #commitAt(
    state: BridgeConnectionState,
    reason: BridgeConnectionReason,
    at: number,
    reconnect: BridgeReconnectSchedule | null,
  ): BridgeConnectionTransitionReceipt {
    const previous = this.#status;
    const current = { state, reason } as BridgeConnectionStatus;
    this.#sequence = checkedIncrement(this.#sequence);
    this.#status = current;
    return {
      sequence: this.#sequence,
      at,
      previous,
      current,
      sessionId: this.#session?.receipt.sessionId ?? null,
      reconnect,
    };
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
