import { CheckedSnapshotConnection } from "@inflatable-cookie/longhorn/core";
import type {
  ConnectionFailure,
  ConnectionFailureReporter,
  SnapshotEventAction,
} from "@inflatable-cookie/longhorn/core";

import {
  assertValidNativeContentChangedEvent,
  assertValidNativeContentConnectRequest,
  assertValidNativeContentConnectResult,
  assertValidNativeContentDecisionRequest,
  assertValidNativeContentDecisionResult,
  assertValidNativeContentDesiredUpdateRequest,
  assertValidNativeContentDesiredUpdateResult,
  assertValidNativeContentSnapshot,
  assertValidNativeContentSnapshotRequest,
  assertValidNativeContentSnapshotResult,
} from "./validation.ts";
import { NATIVE_CONTENT_PROTOCOL_VERSION } from "./generated/protocol.ts";
import type {
  ContentSizeDecision,
  ContentSizeProposal,
  DesiredUpdate,
  NativeContentContentSizeDecisionResult,
  NativeContentDesiredUpdateResult,
  NativeContentIslandId,
  NativeContentProtocolRejection,
  NativeContentSnapshot,
} from "./generated/protocol.ts";
import type { NativeContentPort } from "./ports.ts";

const DEFAULT_MAXIMUM_PENDING_REQUESTS = 32;

export interface NativeContentClientOptions {
  readonly maximumPendingRequests?: number;
}

export interface NativeContentConnection {
  readonly ready: Promise<NativeContentSnapshot>;
  current(): NativeContentSnapshot | undefined;
  failures(): ReadonlyArray<ConnectionFailure>;
  updateDesired(update: DesiredUpdate): Promise<NativeContentDesiredUpdateResult>;
  decideContentSize(
    proposal: ContentSizeProposal,
    decision: ContentSizeDecision,
  ): Promise<NativeContentContentSizeDecisionResult>;
  dispose(): Promise<void>;
}

export class NativeContentClient {
  readonly #port: NativeContentPort;
  readonly #islandId: NativeContentIslandId;
  readonly #maximumPendingRequests: number;

  constructor(
    port: NativeContentPort,
    islandId: NativeContentIslandId,
    options: NativeContentClientOptions = {},
  ) {
    const maximum = options.maximumPendingRequests ?? DEFAULT_MAXIMUM_PENDING_REQUESTS;
    if (!Number.isSafeInteger(maximum) || maximum < 1) {
      throw new RangeError("maximumPendingRequests must be a positive integer");
    }
    this.#port = port;
    this.#islandId = islandId;
    this.#maximumPendingRequests = maximum;
  }

  connect(
    listener?: (snapshot: NativeContentSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): NativeContentConnection {
    return new CheckedNativeContentConnection(
      this.#port,
      this.#islandId,
      this.#maximumPendingRequests,
      listener,
      onFailure,
    );
  }
}

/**
 * Native-content lifetime on the shared snapshot connection: listener first,
 * connect for the first authoritative snapshot, then refresh on events. This
 * class adds only what is native-content specific: the connect/snapshot
 * requests, the event cursor rules and the bounded mutation requests.
 */
class CheckedNativeContentConnection implements NativeContentConnection {
  readonly ready: Promise<NativeContentSnapshot>;
  readonly #port: NativeContentPort;
  readonly #islandId: NativeContentIslandId;
  readonly #maximumPendingRequests: number;
  readonly #connection: CheckedSnapshotConnection<NativeContentSnapshot>;
  readonly #pending = new Set<string>();
  #disposed = false;

  constructor(
    port: NativeContentPort,
    islandId: NativeContentIslandId,
    maximumPendingRequests: number,
    listener?: (snapshot: NativeContentSnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ) {
    this.#port = port;
    this.#islandId = islandId;
    this.#maximumPendingRequests = maximumPendingRequests;
    this.#connection = new CheckedSnapshotConnection<NativeContentSnapshot>({
      listen: async (receive) => (port.listen === undefined ? () => {} : port.listen(receive)),
      loadSnapshot: () => this.#loadSnapshot(),
      validateSnapshot: parseSnapshot,
      handleEvent: (payload, current) => eventAction(payload, current, islandId),
      isNewer: isNewerSessionSnapshot,
      onSnapshot: listener,
      onFailure,
      disposedBeforeReadyError: () => new NativeContentConnectionDisposedError(),
    });
    this.ready = this.#connection.ready.then((snapshot) => {
      if (snapshot === undefined) {
        throw new NativeContentConnectionUnavailableError();
      }
      return snapshot;
    });
  }

  current(): NativeContentSnapshot | undefined {
    return this.#connection.current();
  }

  failures(): ReadonlyArray<ConnectionFailure> {
    return this.#connection.failures();
  }

  async updateDesired(update: DesiredUpdate): Promise<NativeContentDesiredUpdateResult> {
    const current = await this.#requireCurrent();
    const requestId = this.#beginRequest();
    try {
      const request = {
        protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
        request_id: requestId,
        island_id: this.#islandId,
        authority_epoch: current.cursor.authority_epoch,
        client_epoch: current.cursor.client_epoch,
        expected_desired_revision: current.cursor.desired_revision,
        update,
      };
      assertValidNativeContentDesiredUpdateRequest(request);
      const value = await this.#port.updateDesired(request);
      assertValidNativeContentDesiredUpdateResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "committed") {
        this.#connection.offer(value.snapshot);
      }
      return value;
    } finally {
      this.#pending.delete(requestId);
    }
  }

  async decideContentSize(
    proposal: ContentSizeProposal,
    decision: ContentSizeDecision,
  ): Promise<NativeContentContentSizeDecisionResult> {
    const current = await this.#requireCurrent();
    const requestId = this.#beginRequest();
    try {
      const request = {
        protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
        request_id: requestId,
        island_id: this.#islandId,
        authority_epoch: current.cursor.authority_epoch,
        client_epoch: current.cursor.client_epoch,
        proposal,
        decision,
      };
      assertValidNativeContentDecisionRequest(request);
      const value = await this.#port.decideContentSize(request);
      assertValidNativeContentDecisionResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "decided") {
        this.#connection.offer(value.snapshot);
      }
      return value;
    } finally {
      this.#pending.delete(requestId);
    }
  }

  async dispose(): Promise<void> {
    this.#disposed = true;
    this.#pending.clear();
    try {
      await this.#connection.dispose();
    } catch (error) {
      // A failed start is already retained in failures(). Disposing the
      // connection is not a second failure, so only a teardown error rejects.
      const reported = this.#connection
        .failures()
        .some((failure) => failure.phase !== "unlisten" && failure.error === error);
      if (!reported) {
        throw error;
      }
    }
  }

  async #loadSnapshot(): Promise<unknown> {
    const current = this.#connection.current();
    return current === undefined ? this.#connect() : this.#refresh(current);
  }

  async #connect(): Promise<unknown> {
    const requestId = this.#port.nextRequestId();
    const request = {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      request_id: requestId,
      island_id: this.#islandId,
    };
    assertValidNativeContentConnectRequest(request);
    const value = await this.#port.connect(request);
    assertValidNativeContentConnectResult(value);
    assertCorrelation(value.request_id, requestId);
    if (value.status === "rejected") {
      throw new NativeContentRequestRejectedError(value.rejection);
    }
    return value.snapshot;
  }

  async #refresh(current: NativeContentSnapshot): Promise<unknown> {
    const requestId = this.#port.nextRequestId();
    const request = {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      request_id: requestId,
      island_id: this.#islandId,
      authority_epoch: current.cursor.authority_epoch,
      client_epoch: current.cursor.client_epoch,
    };
    assertValidNativeContentSnapshotRequest(request);
    const value = await this.#port.snapshot(request);
    assertValidNativeContentSnapshotResult(value);
    assertCorrelation(value.request_id, requestId);
    if (value.status === "rejected") {
      throw new NativeContentRequestRejectedError(value.rejection);
    }
    return value.snapshot;
  }

  async #requireCurrent(): Promise<NativeContentSnapshot> {
    await this.ready;
    if (this.#disposed) {
      throw new NativeContentConnectionDisposedError();
    }
    const current = this.#connection.current();
    if (current === undefined) {
      throw new NativeContentConnectionUnavailableError();
    }
    return current;
  }

  #beginRequest(): string {
    if (this.#pending.size >= this.#maximumPendingRequests) {
      throw new NativeContentPendingRequestLimitError(this.#maximumPendingRequests);
    }
    const requestId = this.#port.nextRequestId();
    if (this.#pending.has(requestId)) {
      throw new NativeContentDuplicateRequestIdError(requestId);
    }
    this.#pending.add(requestId);
    return requestId;
  }
}

function parseSnapshot(value: unknown): NativeContentSnapshot {
  assertValidNativeContentSnapshot(value);
  return value;
}

function eventAction(
  payload: unknown,
  current: NativeContentSnapshot | undefined,
  islandId: NativeContentIslandId,
): SnapshotEventAction {
  assertValidNativeContentChangedEvent(payload);
  const event = payload.cursor;
  if (event.island_id !== islandId) {
    return { kind: "ignore" };
  }
  if (current === undefined) {
    return { kind: "refresh" };
  }

  const currentCursor = current.cursor;
  if (event.authority_epoch < currentCursor.authority_epoch) {
    return { kind: "ignore" };
  }
  if (
    event.authority_epoch > currentCursor.authority_epoch ||
    event.client_epoch > currentCursor.client_epoch
  ) {
    throw new NativeContentSessionReplacedError();
  }
  if (event.client_epoch < currentCursor.client_epoch) {
    return { kind: "ignore" };
  }
  if (
    event.desired_revision <= currentCursor.desired_revision &&
    event.observed_revision <= currentCursor.observed_revision
  ) {
    return { kind: "ignore" };
  }
  return { kind: "refresh" };
}

/**
 * A snapshot only replaces the current one within the same authority and
 * client epoch. A different session cannot arrive by refresh or by request
 * result: an authority or client epoch change is a session replacement, which
 * the event path reports as a failure.
 */
function isNewerSessionSnapshot(
  candidate: NativeContentSnapshot,
  current: NativeContentSnapshot | undefined,
): boolean {
  if (current === undefined) {
    return true;
  }
  if (
    candidate.cursor.authority_epoch !== current.cursor.authority_epoch ||
    candidate.cursor.client_epoch !== current.cursor.client_epoch
  ) {
    return false;
  }
  return isNewerNativeContentSnapshot(candidate, current);
}

export function isNewerNativeContentSnapshot(
  candidate: NativeContentSnapshot,
  current: NativeContentSnapshot | undefined,
): boolean {
  if (current === undefined) {
    return true;
  }
  if (candidate.cursor.authority_epoch !== current.cursor.authority_epoch) {
    return candidate.cursor.authority_epoch > current.cursor.authority_epoch;
  }
  if (candidate.cursor.client_epoch !== current.cursor.client_epoch) {
    return false;
  }
  if (
    candidate.cursor.desired_revision < current.cursor.desired_revision ||
    candidate.cursor.observed_revision < current.cursor.observed_revision
  ) {
    return false;
  }
  return (
    candidate.cursor.desired_revision > current.cursor.desired_revision ||
    candidate.cursor.observed_revision > current.cursor.observed_revision
  );
}

function assertCorrelation(received: string, expected: string): void {
  if (received !== expected) {
    throw new NativeContentResponseCorrelationError(expected, received);
  }
}

export class NativeContentResponseCorrelationError extends Error {
  constructor(
    readonly expectedRequestId: string,
    readonly receivedRequestId: string,
  ) {
    super(
      `native-content response correlation mismatch: expected ${expectedRequestId}; received ${receivedRequestId}`,
    );
    this.name = "NativeContentResponseCorrelationError";
  }
}

export class NativeContentRequestRejectedError extends Error {
  constructor(readonly rejection: NativeContentProtocolRejection) {
    super(rejection.message);
    this.name = "NativeContentRequestRejectedError";
  }
}

export class NativeContentConnectionDisposedError extends Error {
  constructor() {
    super("native-content connection was disposed");
    this.name = "NativeContentConnectionDisposedError";
  }
}

export class NativeContentConnectionUnavailableError extends Error {
  constructor() {
    super("native-content connection has no authoritative snapshot");
    this.name = "NativeContentConnectionUnavailableError";
  }
}

export class NativeContentSessionReplacedError extends Error {
  constructor() {
    super("native-content renderer session was replaced");
    this.name = "NativeContentSessionReplacedError";
  }
}

export class NativeContentPendingRequestLimitError extends Error {
  constructor(readonly maximum: number) {
    super(`native-content pending request limit reached: ${maximum}`);
    this.name = "NativeContentPendingRequestLimitError";
  }
}

export class NativeContentDuplicateRequestIdError extends Error {
  constructor(readonly requestId: string) {
    super(`native-content request id is already pending: ${requestId}`);
    this.name = "NativeContentDuplicateRequestIdError";
  }
}
