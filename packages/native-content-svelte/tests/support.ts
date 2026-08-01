import fixture from "../../../fixtures/native-content/protocol-v1.json";

import {
  NATIVE_CONTENT_PROTOCOL_VERSION,
  type ContentSizeDecision,
  type ContentSizeProposal,
  type DesiredUpdate,
  type NativeContentChangedEvent,
  type NativeContentConnection,
  type NativeContentContentSizeDecisionResult,
  type NativeContentDesiredUpdateResult,
  type NativeContentSnapshot,
} from "@longhorn/native-content";

import type {
  NativeContentResizeObserver,
  NativeContentResizeObserverFactory,
  NativeContentSessionClient,
} from "../src/index.ts";

export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}

export function baseSnapshot(): NativeContentSnapshot {
  return clone(fixture.connect.snapshot) as NativeContentSnapshot;
}

export function deferred<Value>() {
  let resolve!: (value: Value) => void;
  const promise = new Promise<Value>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

export class ScriptedClient implements NativeContentSessionClient {
  readonly connections: ScriptedConnection[] = [];

  connect(
    listener?: (snapshot: NativeContentSnapshot) => void,
  ): NativeContentConnection {
    const snapshot = baseSnapshot();
    snapshot.cursor.client_epoch = this.connections.length + 1;
    const connection = new ScriptedConnection(snapshot, listener);
    this.connections.push(connection);
    return connection;
  }
}

export class ScriptedConnection implements NativeContentConnection {
  readonly ready: Promise<NativeContentSnapshot>;
  readonly updates: DesiredUpdate[] = [];
  disposeCount = 0;
  #snapshot: NativeContentSnapshot;
  #listener: ((snapshot: NativeContentSnapshot) => void) | undefined;
  #nextGate: Promise<void> | undefined;
  #nextResult: Promise<NativeContentDesiredUpdateResult> | undefined;
  #request = 0;
  #disposed = false;

  constructor(
    snapshot: NativeContentSnapshot,
    listener?: (snapshot: NativeContentSnapshot) => void,
  ) {
    this.#snapshot = clone(snapshot);
    this.#listener = listener;
    this.ready = Promise.resolve(this.currentSnapshot());
  }

  current(): NativeContentSnapshot | undefined {
    return this.currentSnapshot();
  }

  currentSnapshot(): NativeContentSnapshot {
    return clone(this.#snapshot);
  }

  get listenerAttached(): boolean {
    return this.#listener !== undefined;
  }

  failures(): readonly [] {
    return [];
  }

  blockNextUpdate(): () => void {
    const gate = deferred<void>();
    this.#nextGate = gate.promise;
    return () => gate.resolve();
  }

  resolveNextUpdateWith(
    result: Promise<NativeContentDesiredUpdateResult>,
  ): void {
    this.#nextResult = result;
  }

  advanceGeneration(generation: number): NativeContentSnapshot {
    const previous = this.#snapshot.cursor.desired_revision;
    this.#snapshot.cursor.attach_generation = generation;
    this.#snapshot.cursor.desired_revision = previous + 1;
    this.#snapshot.desired = {
      ...this.#snapshot.desired,
      revision: previous + 1,
      generation,
    };
    const snapshot = this.currentSnapshot();
    if (!this.#disposed) this.#listener?.(snapshot);
    return snapshot;
  }

  async updateDesired(
    update: DesiredUpdate,
  ): Promise<NativeContentDesiredUpdateResult> {
    this.updates.push(clone(update));
    const fixedResult = this.#nextResult;
    this.#nextResult = undefined;
    if (fixedResult !== undefined) return fixedResult;

    const gate = this.#nextGate;
    this.#nextGate = undefined;
    await gate;
    return this.#commit(update);
  }

  async decideContentSize(
    _proposal: ContentSizeProposal,
    _decision: ContentSizeDecision,
  ): Promise<NativeContentContentSizeDecisionResult> {
    throw new Error("content-size decisions are outside this Svelte fixture");
  }

  async dispose(): Promise<void> {
    if (this.#disposed) return;
    this.#disposed = true;
    this.#listener = undefined;
    this.disposeCount += 1;
  }

  #commit(update: DesiredUpdate): NativeContentDesiredUpdateResult {
    const previous = this.#snapshot.cursor.desired_revision;
    const current = previous + 1;
    this.#snapshot.cursor.desired_revision = current;
    this.#snapshot.cursor.attach_generation = update.generation;
    this.#snapshot.desired = {
      ...clone(this.#snapshot.desired),
      ...clone(update),
      revision: current,
    };
    const requestId = `request:svelte-${++this.#request}`;
    const receipt = {
      previous_revision: previous,
      current_revision: current,
      generation: update.generation,
    };
    const event: NativeContentChangedEvent = {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      cursor: clone(this.#snapshot.cursor),
      change: {
        kind: "desired_updated",
        request_id: requestId,
        receipt,
      },
    };
    const snapshot = this.currentSnapshot();
    if (!this.#disposed) this.#listener?.(snapshot);
    return {
      status: "committed",
      request_id: requestId,
      snapshot,
      receipt,
      event,
    };
  }
}

export class ResizeObserverTrace {
  readonly callbacks: Array<() => void> = [];
  readonly observers: TracedResizeObserver[] = [];
  readonly factory: NativeContentResizeObserverFactory = (callback) => {
    this.callbacks.push(callback);
    const observer = new TracedResizeObserver();
    this.observers.push(observer);
    return observer;
  };

  fire(index = this.callbacks.length - 1): void {
    this.callbacks[index]?.();
  }
}

class TracedResizeObserver implements NativeContentResizeObserver {
  readonly observed: Element[] = [];
  disconnectCount = 0;

  observe(target: Element): void {
    this.observed.push(target);
  }

  disconnect(): void {
    this.disconnectCount += 1;
  }
}

export function viewportElement(
  rect: { left: number; top: number; width: number; height: number },
): HTMLElement {
  const element = document.createElement("div");
  element.getBoundingClientRect = () =>
    ({
      ...rect,
      x: rect.left,
      y: rect.top,
      right: rect.left + rect.width,
      bottom: rect.top + rect.height,
      toJSON: () => rect,
    }) as DOMRect;
  return element;
}

export function committedResult(
  snapshot: NativeContentSnapshot,
): NativeContentDesiredUpdateResult {
  const requestId = "request:stale-result";
  const receipt = {
    previous_revision: snapshot.cursor.desired_revision - 1,
    current_revision: snapshot.cursor.desired_revision,
    generation: snapshot.cursor.attach_generation,
  };
  return {
    status: "committed",
    request_id: requestId,
    snapshot: clone(snapshot),
    receipt,
    event: {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      cursor: clone(snapshot.cursor),
      change: {
        kind: "desired_updated",
        request_id: requestId,
        receipt,
      },
    },
  };
}
