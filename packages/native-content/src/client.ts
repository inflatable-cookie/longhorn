import type {
  ConnectionFailure,
  ConnectionFailureReporter,
} from "@longhorn/core";

import {
  assertCompatibleNativeContentChangedEvent,
  assertCompatibleNativeContentConnectRequest,
  assertCompatibleNativeContentConnectResult,
  assertCompatibleNativeContentDecisionRequest,
  assertCompatibleNativeContentDecisionResult,
  assertCompatibleNativeContentDesiredUpdateRequest,
  assertCompatibleNativeContentDesiredUpdateResult,
  assertCompatibleNativeContentSnapshotRequest,
  assertCompatibleNativeContentSnapshotResult,
} from "./compatibility.ts";
import {
  NATIVE_CONTENT_PROTOCOL_VERSION,
  type ContentSizeDecision,
  type ContentSizeProposal,
  type DesiredUpdate,
  type NativeContentChangedEvent,
  type NativeContentContentSizeDecisionResult,
  type NativeContentDesiredUpdateResult,
  type NativeContentIslandId,
  type NativeContentSnapshot,
} from "./generated/protocol.ts";
import type { NativeContentPort, NativeContentUnlisten } from "./ports.ts";

const DEFAULT_MAXIMUM_PENDING_REQUESTS = 32;

export interface NativeContentClientOptions {
  readonly maximumPendingRequests?: number;
}

export interface NativeContentConnection {
  readonly ready: Promise<NativeContentSnapshot>;
  current(): NativeContentSnapshot | undefined;
  failures(): readonly ConnectionFailure[];
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
    const maximum =
      options.maximumPendingRequests ?? DEFAULT_MAXIMUM_PENDING_REQUESTS;
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

class CheckedNativeContentConnection implements NativeContentConnection {
  readonly ready: Promise<NativeContentSnapshot>;
  readonly #port: NativeContentPort;
  readonly #islandId: NativeContentIslandId;
  readonly #maximumPendingRequests: number;
  readonly #listener: ((snapshot: NativeContentSnapshot) => void) | undefined;
  readonly #onFailure: ConnectionFailureReporter | undefined;
  readonly #failures: ConnectionFailure[] = [];
  readonly #pending = new Set<string>();
  readonly #queuedEvents: NativeContentChangedEvent[] = [];
  #snapshot: NativeContentSnapshot | undefined;
  #unlisten: NativeContentUnlisten | undefined;
  #cleanup: Promise<void> | undefined;
  #pump: Promise<void> | undefined;
  #refreshPending = false;
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
    this.#listener = listener;
    this.#onFailure = onFailure;
    this.ready = this.#start();
  }

  current(): NativeContentSnapshot | undefined {
    return this.#snapshot;
  }

  failures(): readonly ConnectionFailure[] {
    return [...this.#failures];
  }

  async updateDesired(
    update: DesiredUpdate,
  ): Promise<NativeContentDesiredUpdateResult> {
    const current = await this.#requireCurrent();
    const requestId = this.#beginRequest();
    const session = sessionCursor(current);
    const request = {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      request_id: requestId,
      island_id: this.#islandId,
      client_epoch: current.cursor.client_epoch,
      expected_desired_revision: current.cursor.desired_revision,
      update,
    };
    assertCompatibleNativeContentDesiredUpdateRequest(request);
    try {
      const value = await this.#port.updateDesired(request);
      assertCompatibleNativeContentDesiredUpdateResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "committed") this.#offer(value.snapshot, session);
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
    const session = sessionCursor(current);
    const request = {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      request_id: requestId,
      island_id: this.#islandId,
      client_epoch: current.cursor.client_epoch,
      proposal,
      decision,
    };
    assertCompatibleNativeContentDecisionRequest(request);
    try {
      const value = await this.#port.decideContentSize(request);
      assertCompatibleNativeContentDecisionResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "decided") this.#offer(value.snapshot, session);
      return value;
    } finally {
      this.#pending.delete(requestId);
    }
  }

  async dispose(): Promise<void> {
    this.#disposed = true;
    this.#pending.clear();
    await this.#closeListener();
    try {
      await this.ready;
    } catch {
      // Registration/start failures are already retained in failures().
    }
    await this.#closeListener();
  }

  async #start(): Promise<NativeContentSnapshot> {
    try {
      this.#unlisten =
        this.#port.listen === undefined
          ? () => {}
          : await this.#port.listen((value) => this.#handleEvent(value));
    } catch (error) {
      this.#recordFailure("registration", error);
      throw error;
    }

    if (this.#disposed) {
      await this.#closeListener();
      throw new NativeContentConnectionDisposedError();
    }

    try {
      const requestId = this.#port.nextRequestId();
      const request = {
        protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
        request_id: requestId,
        island_id: this.#islandId,
      };
      assertCompatibleNativeContentConnectRequest(request);
      const value = await this.#port.connect(request);
      assertCompatibleNativeContentConnectResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "rejected") {
        throw new NativeContentRequestRejectedError(value.rejection);
      }
      if (this.#disposed) {
        await this.#closeListener();
        throw new NativeContentConnectionDisposedError();
      }
      this.#install(value.snapshot);
      for (const event of this.#queuedEvents.splice(0)) this.#acceptEvent(event);
      await this.#synchronize();
      const current = this.#snapshot;
      if (current === undefined) throw new Error("native-content connect returned no snapshot");
      return current;
    } catch (error) {
      this.#recordFailure("snapshot", error);
      await this.#closeAfterFailure();
      throw error;
    }
  }

  #handleEvent(value: unknown): void {
    if (this.#disposed || this.#failures.length > 0) return;
    try {
      assertCompatibleNativeContentChangedEvent(value);
      if (value.cursor.island_id !== this.#islandId) return;
      if (this.#snapshot === undefined) {
        this.#queuedEvents.push(value);
        return;
      }
      this.#acceptEvent(value);
    } catch (error) {
      this.#recordFailure("event", error);
      void this.#closeAfterFailure();
    }
  }

  #acceptEvent(event: NativeContentChangedEvent): void {
    const current = this.#snapshot;
    if (current === undefined) return;
    const currentCursor = current.cursor;
    if (event.cursor.authority_epoch < currentCursor.authority_epoch) return;
    if (
      event.cursor.authority_epoch > currentCursor.authority_epoch ||
      event.cursor.client_epoch > currentCursor.client_epoch
    ) {
      throw new NativeContentSessionReplacedError();
    }
    if (event.cursor.client_epoch < currentCursor.client_epoch) return;
    if (
      event.cursor.desired_revision <= currentCursor.desired_revision &&
      event.cursor.observed_revision <= currentCursor.observed_revision
    ) {
      return;
    }
    this.#refreshPending = true;
    void this.#synchronize().catch((error) => {
      this.#recordFailure("snapshot", error);
      void this.#closeAfterFailure();
    });
  }

  #synchronize(): Promise<void> {
    if (this.#disposed || this.#failures.length > 0 || !this.#refreshPending) {
      return Promise.resolve();
    }
    this.#pump ??= this.#runPump().finally(() => {
      this.#pump = undefined;
    });
    return this.#pump;
  }

  async #runPump(): Promise<void> {
    while (this.#refreshPending && !this.#disposed && this.#failures.length === 0) {
      this.#refreshPending = false;
      const current = this.#snapshot;
      if (current === undefined) return;
      const requestId = this.#port.nextRequestId();
      const request = {
        protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
        request_id: requestId,
        island_id: this.#islandId,
        client_epoch: current.cursor.client_epoch,
      };
      assertCompatibleNativeContentSnapshotRequest(request);
      const value = await this.#port.snapshot(request);
      assertCompatibleNativeContentSnapshotResult(value);
      assertCorrelation(value.request_id, requestId);
      if (value.status === "rejected") {
        throw new NativeContentRequestRejectedError(value.rejection);
      }
      this.#offer(value.snapshot, sessionCursor(current));
    }
  }

  #offer(snapshot: NativeContentSnapshot, session: SessionCursor): void {
    if (this.#disposed) return;
    const current = this.#snapshot;
    if (current === undefined) return;
    if (
      current.cursor.authority_epoch !== session.authorityEpoch ||
      current.cursor.client_epoch !== session.clientEpoch ||
      snapshot.cursor.authority_epoch !== session.authorityEpoch ||
      snapshot.cursor.client_epoch !== session.clientEpoch
    ) {
      return;
    }
    if (!isNewerSnapshot(snapshot, current)) return;
    this.#install(snapshot);
  }

  #install(snapshot: NativeContentSnapshot): void {
    this.#snapshot = snapshot;
    this.#listener?.(snapshot);
  }

  async #requireCurrent(): Promise<NativeContentSnapshot> {
    await this.ready;
    if (this.#disposed) throw new NativeContentConnectionDisposedError();
    const current = this.#snapshot;
    if (current === undefined) throw new NativeContentConnectionUnavailableError();
    return current;
  }

  #beginRequest(): string {
    if (this.#pending.size >= this.#maximumPendingRequests) {
      throw new NativeContentPendingRequestLimitError(
        this.#maximumPendingRequests,
      );
    }
    const requestId = this.#port.nextRequestId();
    if (this.#pending.has(requestId)) {
      throw new NativeContentDuplicateRequestIdError(requestId);
    }
    this.#pending.add(requestId);
    return requestId;
  }

  #recordFailure(phase: ConnectionFailure["phase"], error: unknown): void {
    if (this.#failures.some((failure) => failure.phase === phase && failure.error === error)) return;
    const failure = { phase, error };
    this.#failures.push(failure);
    this.#onFailure?.(failure);
  }

  async #closeAfterFailure(): Promise<void> {
    try {
      await this.#closeListener();
    } catch (error) {
      this.#recordFailure("unlisten", error);
    }
  }

  #closeListener(): Promise<void> {
    if (this.#cleanup !== undefined) return this.#cleanup;
    const unlisten = this.#unlisten;
    if (unlisten === undefined) return Promise.resolve();
    this.#unlisten = undefined;
    this.#cleanup = Promise.resolve().then(() => unlisten());
    return this.#cleanup;
  }
}

interface SessionCursor {
  readonly authorityEpoch: number;
  readonly clientEpoch: number;
}

function sessionCursor(snapshot: NativeContentSnapshot): SessionCursor {
  return {
    authorityEpoch: snapshot.cursor.authority_epoch,
    clientEpoch: snapshot.cursor.client_epoch,
  };
}

export function isNewerNativeContentSnapshot(
  candidate: NativeContentSnapshot,
  current: NativeContentSnapshot | undefined,
): boolean {
  if (current === undefined) return true;
  if (candidate.cursor.authority_epoch !== current.cursor.authority_epoch) {
    return candidate.cursor.authority_epoch > current.cursor.authority_epoch;
  }
  if (candidate.cursor.client_epoch !== current.cursor.client_epoch) return false;
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

function isNewerSnapshot(
  candidate: NativeContentSnapshot,
  current: NativeContentSnapshot,
): boolean {
  return isNewerNativeContentSnapshot(candidate, current);
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
  constructor(readonly rejection: import("./generated/protocol.ts").NativeContentProtocolRejection) {
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
