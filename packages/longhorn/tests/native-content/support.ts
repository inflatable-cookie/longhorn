import fixture from "../../../../fixtures/native-content/protocol-v1.json";

import {
  createDirectNativeContentPort,
  NATIVE_CONTENT_PROTOCOL_VERSION,
  type DesiredUpdate,
  type NativeContentChangedEvent,
  type NativeContentConnectRequest,
  type NativeContentContentSizeDecisionRequest,
  type NativeContentDesiredUpdateRequest,
  type NativeContentPort,
  type NativeContentProtocolRejection,
  type NativeContentSnapshot,
  type NativeContentSnapshotRequest,
} from "../../src/native-content/index.ts";

export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}

export function baseSnapshot(): NativeContentSnapshot {
  return clone(fixture.connect.snapshot) as NativeContentSnapshot;
}

export function nextUpdate(
  snapshot: NativeContentSnapshot,
  x: number,
  attachGeneration = snapshot.cursor.attach_generation,
): DesiredUpdate {
  return {
    generation: attachGeneration,
    host_window_id: snapshot.desired.host_window_id,
    viewport: {
      ...clone(snapshot.desired.viewport),
      origin: { x, y: snapshot.desired.viewport.origin.y },
    },
    scale: snapshot.desired.scale,
    rounding: snapshot.desired.rounding,
    presence: snapshot.desired.presence,
    visibility: clone(snapshot.desired.visibility),
    focus: snapshot.desired.focus,
    input_routing: snapshot.desired.input_routing,
  };
}

export class MemoryNativeContentHost {
  readonly calls: string[] = [];
  readonly listeners = new Set<(event: unknown) => void>();
  unlistenCount = 0;
  beforeConnectReturn: (() => void) | undefined;
  #snapshot = baseSnapshot();
  #requestSequence = 0;
  #clientEpoch = 0;

  port(): NativeContentPort {
    return createDirectNativeContentPort({
      connect: (request) => this.connect(request),
      snapshot: (request) => this.snapshot(request),
      updateDesired: (request) => this.updateDesired(request),
      decideContentSize: (request) => this.decideContentSize(request),
      listen: (listener) => this.listen(listener),
      nextRequestId: () => this.nextRequestId(),
    });
  }

  current(): NativeContentSnapshot {
    return clone(this.#snapshot);
  }

  async connect(request: NativeContentConnectRequest): Promise<unknown> {
    this.calls.push("connect");
    this.#clientEpoch += 1;
    this.#snapshot.cursor.client_epoch = this.#clientEpoch;
    const connected = clone(this.#snapshot);
    this.beforeConnectReturn?.();
    this.beforeConnectReturn = undefined;
    return {
      status: "connected",
      request_id: request.request_id,
      snapshot: connected,
    };
  }

  async snapshot(request: NativeContentSnapshotRequest): Promise<unknown> {
    this.calls.push("snapshot");
    if (request.client_epoch !== this.#clientEpoch) {
      return {
        status: "rejected",
        request_id: request.request_id,
        rejection: staleSessionRejection(),
      };
    }
    return {
      status: "ready",
      request_id: request.request_id,
      snapshot: this.current(),
    };
  }

  async updateDesired(
    request: NativeContentDesiredUpdateRequest,
  ): Promise<unknown> {
    this.calls.push("update");
    if (request.client_epoch !== this.#clientEpoch) {
      return {
        status: "rejected",
        request_id: request.request_id,
        rejection: staleSessionRejection(),
      };
    }
    if (
      request.expected_desired_revision !==
      this.#snapshot.cursor.desired_revision
    ) {
      return {
        status: "rejected",
        request_id: request.request_id,
        rejection: staleRevisionRejection(),
      };
    }
    const previous = this.#snapshot.cursor.desired_revision;
    const current = previous + 1;
    this.#snapshot.desired = {
      ...clone(this.#snapshot.desired),
      ...clone(request.update),
      revision: current,
    };
    this.#snapshot.cursor = {
      ...this.#snapshot.cursor,
      attach_generation: request.update.generation,
      desired_revision: current,
    };
    const receipt = {
      previous_revision: previous,
      current_revision: current,
      generation: request.update.generation,
    };
    const event = this.event({
      kind: "desired_updated",
      request_id: request.request_id,
      receipt,
    });
    this.emit(event);
    return {
      status: "committed",
      request_id: request.request_id,
      snapshot: this.current(),
      receipt,
      event,
    };
  }

  async decideContentSize(
    request: NativeContentContentSizeDecisionRequest,
  ): Promise<unknown> {
    this.calls.push("decide");
    const acceptedSize =
      request.decision.kind === "accepted"
        ? request.proposal.size
        : request.decision.kind === "constrained"
          ? request.decision.size
          : null;
    const receipt = {
      proposal: clone(request.proposal),
      decision: clone(request.decision),
      accepted_size: clone(acceptedSize),
    };
    const event = this.event({
      kind: "content_size_decided",
      request_id: request.request_id,
      receipt,
    });
    this.emit(event);
    return {
      status: "decided",
      request_id: request.request_id,
      snapshot: this.current(),
      receipt,
      event,
    };
  }

  listen(listener: (event: unknown) => void): () => void {
    this.calls.push("listen");
    this.listeners.add(listener);
    return () => {
      if (this.listeners.delete(listener)) this.unlistenCount += 1;
    };
  }

  nextRequestId(): string {
    this.#requestSequence += 1;
    return `request:ts-${this.#requestSequence}`;
  }

  admitObservation(): void {
    const previous = this.#snapshot.cursor.observed_revision;
    const current = previous + 1;
    this.#snapshot.observed = {
      ...this.#snapshot.observed,
      revision: current,
      lifecycle: "attaching",
      readiness: "not_ready",
    };
    this.#snapshot.cursor.observed_revision = current;
    this.emit(
      this.event({
        kind: "observation_admitted",
        request_id: "request:host-observe",
        receipt: {
          previous_revision: previous,
          current_revision: current,
          generation: this.#snapshot.cursor.attach_generation,
          lifecycle: "attaching",
        },
      }),
    );
  }

  replaceDesired(x: number, attachGeneration: number): void {
    const previous = this.#snapshot.cursor.desired_revision;
    const current = previous + 1;
    const update = nextUpdate(this.#snapshot, x, attachGeneration);
    this.#snapshot.desired = {
      ...this.#snapshot.desired,
      ...update,
      revision: current,
    };
    this.#snapshot.cursor.attach_generation = attachGeneration;
    this.#snapshot.cursor.desired_revision = current;
    this.emit(
      this.event({
        kind: "desired_updated",
        request_id: "request:host-update",
        receipt: {
          previous_revision: previous,
          current_revision: current,
          generation: attachGeneration,
        },
      }),
    );
  }

  emit(event: NativeContentChangedEvent): void {
    for (const listener of this.listeners) listener(clone(event));
  }

  event(change: NativeContentChangedEvent["change"]): NativeContentChangedEvent {
    return {
      protocol_version: NATIVE_CONTENT_PROTOCOL_VERSION,
      cursor: clone(this.#snapshot.cursor),
      change,
    };
  }
}

export async function flush(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

function staleSessionRejection(): NativeContentProtocolRejection {
  return {
    code: "stale_client_epoch",
    message: "renderer epoch is stale",
    phase: "admission",
    retry: "refresh",
  };
}

function staleRevisionRejection(): NativeContentProtocolRejection {
  return {
    code: "stale_revision",
    message: "desired revision is stale",
    phase: "coordination",
    retry: "refresh",
  };
}
