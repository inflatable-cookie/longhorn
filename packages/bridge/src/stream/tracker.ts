import type {
  BridgeSnapshotDecision,
  BridgeStreamCursor,
  BridgeStreamDecision,
  DomainId,
  BridgeSessionId,
} from "../generated/protocol.ts";

export class BridgeStreamTracker {
  #sessionId: BridgeSessionId;
  readonly #domainId: DomainId;
  #accepted: BridgeStreamCursor | undefined;
  #pending: BridgeStreamCursor | undefined;
  #requiresSnapshot = true;

  constructor(sessionId: BridgeSessionId, domainId: DomainId) {
    this.#sessionId = sessionId;
    this.#domainId = domainId;
  }

  advanceSession(sessionId: BridgeSessionId): void {
    if (this.#sessionId !== sessionId) {
      this.#sessionId = sessionId;
      this.#accepted = undefined;
      this.#pending = undefined;
      this.#requiresSnapshot = true;
    }
  }

  acceptSnapshot(cursor: BridgeStreamCursor): BridgeSnapshotDecision {
    if (cursor.sessionId !== this.#sessionId) {
      return "supersededSession";
    }
    if (cursor.domainId !== this.#domainId) {
      return "wrongDomain";
    }
    const pendingIsNewer = this.#pending !== undefined &&
      newer(this.#pending, cursor);
    this.#accepted = cursor;
    this.#pending = undefined;
    this.#requiresSnapshot = pendingIsNewer;
    return pendingIsNewer
      ? "acceptedResnapshotRequired"
      : "accepted";
  }

  classifyEvent(cursor: BridgeStreamCursor): BridgeStreamDecision {
    if (cursor.sessionId !== this.#sessionId) {
      return "ignoreSupersededSession";
    }
    if (cursor.domainId !== this.#domainId) {
      return "ignoreWrongDomain";
    }
    if (this.#requiresSnapshot) {
      this.#remember(cursor);
      return "resnapshotRequired";
    }
    const current = this.#accepted;
    if (current === undefined) {
      this.#requiresSnapshot = true;
      this.#remember(cursor);
      return "resnapshotRequired";
    }
    if (cursor.authorityEpoch < current.authorityEpoch) {
      return "ignoreStale";
    }
    if (cursor.authorityEpoch > current.authorityEpoch) {
      this.#requiresSnapshot = true;
      this.#remember(cursor);
      return "resnapshotNewEpoch";
    }
    if (cursor.sequence === current.sequence) {
      return "ignoreDuplicate";
    }
    if (cursor.sequence < current.sequence) {
      return "ignoreStale";
    }
    if (cursor.sequence === current.sequence + 1) {
      this.#accepted = cursor;
      return "apply";
    }
    this.#requiresSnapshot = true;
    this.#remember(cursor);
    return "resnapshotGap";
  }

  acceptedCursor(): BridgeStreamCursor | undefined {
    return this.#accepted;
  }

  requiresSnapshot(): boolean {
    return this.#requiresSnapshot;
  }

  #remember(cursor: BridgeStreamCursor): void {
    if (this.#pending === undefined || newer(cursor, this.#pending)) {
      this.#pending = cursor;
    }
  }
}

export function newer(
  candidate: BridgeStreamCursor,
  current: BridgeStreamCursor,
): boolean {
  return candidate.authorityEpoch > current.authorityEpoch ||
    (candidate.authorityEpoch === current.authorityEpoch &&
      candidate.sequence > current.sequence);
}
