import {
  TRANSFER_PROTOCOL_VERSION,
  type ClientDropZone,
  type TransferClientSnapshot,
  type TransferLeaseRequest,
  type TransferLeaseResponse,
} from "@inflatable-cookie/longhorn-transfer";

import type { TransferState } from "./transfer.svelte.ts";

export type DropZoneDefinition = Omit<ClientDropZone, "bounds">;

export type GeometryObserver = (
  node: HTMLElement,
  changed: () => void,
) => () => void;

export interface DropZoneLeaseRegistryOptions {
  readonly state: TransferState;
  readonly nextRequestId: () => string;
  readonly reportError: (error: unknown) => void;
  readonly observeGeometry?: GeometryObserver;
}

export interface DropZoneAction {
  update(definition: DropZoneDefinition): void;
  destroy(): void;
}

type RegisteredZone = {
  readonly node: HTMLElement;
  definition: DropZoneDefinition;
  stopObserving: () => void;
};

export class DropZoneLeaseRegistry {
  readonly #options: DropZoneLeaseRegistryOptions;
  readonly #zones = new Map<string, RegisteredZone>();
  #tail: Promise<unknown> = Promise.resolve();
  #active = false;
  #destroyed = false;
  #lastGeneration = 0;

  constructor(options: DropZoneLeaseRegistryOptions) {
    this.#options = options;
  }

  readonly zone = (
    node: HTMLElement,
    initialDefinition: DropZoneDefinition,
  ): DropZoneAction => {
    this.#assertAlive();
    this.#assertUnique(initialDefinition.id);
    const registration: RegisteredZone = {
      node,
      definition: initialDefinition,
      stopObserving: () => undefined,
    };
    registration.stopObserving = (
      this.#options.observeGeometry ?? observeElementGeometry
    )(node, () => this.#schedulePublication());
    this.#zones.set(initialDefinition.id, registration);
    this.#schedulePublication();

    return {
      update: (definition) => {
        this.#assertAlive();
        if (definition.id !== registration.definition.id) {
          this.#assertUnique(definition.id);
          this.#zones.delete(registration.definition.id);
          this.#zones.set(definition.id, registration);
        }
        registration.definition = definition;
        this.#schedulePublication();
      },
      destroy: () => {
        registration.stopObserving();
        if (
          this.#zones.get(registration.definition.id) === registration
        ) {
          this.#zones.delete(registration.definition.id);
          this.#schedulePublication();
        }
      },
    };
  };

  async start(): Promise<TransferLeaseResponse> {
    this.#assertAlive();
    this.#active = true;
    return this.publish();
  }

  publish(): Promise<TransferLeaseResponse> {
    return this.#serialize(async () => {
      this.#assertAlive();
      if (!this.#active) {
        throw new DropZoneLeaseNotStartedError();
      }
      return this.#publishCurrent();
    });
  }

  async destroy(): Promise<void> {
    if (this.#destroyed) return;
    this.#active = false;
    this.#destroyed = true;
    for (const registration of this.#zones.values()) {
      registration.stopObserving();
    }
    this.#zones.clear();
    await this.#serialize(() => this.#options.state.releaseLease());
  }

  #schedulePublication(): void {
    if (!this.#active || this.#destroyed) return;
    void this.#serialize(async () => {
      if (!this.#active || this.#destroyed) return;
      await this.#publishCurrent();
    }).catch((error) => this.#report(error));
  }

  async #publishCurrent(): Promise<TransferLeaseResponse> {
    const snapshot = this.#options.state.snapshot;
    if (snapshot === undefined) {
      throw new DropZoneLeaseSnapshotUnavailableError();
    }
    const generation = nextGeneration(snapshot, this.#lastGeneration);
    const request: TransferLeaseRequest = {
      protocol_version: TRANSFER_PROTOCOL_VERSION,
      request_id: this.#options.nextRequestId(),
      client_id: snapshot.client_id,
      client_epoch: snapshot.client_epoch,
      generation,
      zones: [...this.#zones.values()].map(measureZone),
    };
    const response = await this.#options.state.publishLease(request);
    if (response.status === "published") {
      this.#lastGeneration = response.lease.generation;
    }
    return response;
  }

  #serialize<T>(operation: () => Promise<T>): Promise<T> {
    const task = this.#tail.catch(() => undefined).then(operation);
    this.#tail = task;
    return task;
  }

  #assertUnique(id: string): void {
    if (this.#zones.has(id)) {
      throw new DuplicateDropZoneError(id);
    }
  }

  #assertAlive(): void {
    if (this.#destroyed) {
      throw new DropZoneLeaseDestroyedError();
    }
  }

  #report(error: unknown): void {
    try {
      this.#options.reportError(error);
    } catch {
      // Reporting failure must not poison later lease publications.
    }
  }
}

export class DuplicateDropZoneError extends Error {
  constructor(id: string) {
    super(`duplicate drop zone: ${id}`);
    this.name = "DuplicateDropZoneError";
  }
}

export class DropZoneLeaseNotStartedError extends Error {
  constructor() {
    super("drop-zone lease registry must be started before publication");
    this.name = "DropZoneLeaseNotStartedError";
  }
}

export class DropZoneLeaseSnapshotUnavailableError extends Error {
  constructor() {
    super("drop-zone lease publication requires a current client snapshot");
    this.name = "DropZoneLeaseSnapshotUnavailableError";
  }
}

export class DropZoneLeaseDestroyedError extends Error {
  constructor() {
    super("drop-zone lease registry has been destroyed");
    this.name = "DropZoneLeaseDestroyedError";
  }
}

function measureZone(registration: RegisteredZone): ClientDropZone {
  const rect = registration.node.getBoundingClientRect();
  for (const value of [rect.left, rect.top, rect.width, rect.height]) {
    if (!Number.isFinite(value)) {
      throw new RangeError("drop-zone geometry must be finite");
    }
  }
  if (rect.width <= 0 || rect.height <= 0) {
    throw new RangeError("drop-zone geometry must have positive extent");
  }
  return {
    ...registration.definition,
    bounds: {
      origin: { x: rect.left, y: rect.top },
      size: { width: rect.width, height: rect.height },
    },
  };
}

function nextGeneration(
  snapshot: TransferClientSnapshot,
  lastGeneration: number,
): number {
  const generation =
    Math.max(snapshot.current_lease_generation ?? 0, lastGeneration) + 1;
  if (!Number.isSafeInteger(generation)) {
    throw new RangeError("drop-zone lease generation is exhausted");
  }
  return generation;
}

function observeElementGeometry(
  node: HTMLElement,
  changed: () => void,
): () => void {
  if (typeof ResizeObserver === "undefined") {
    return () => undefined;
  }
  const observer = new ResizeObserver(changed);
  observer.observe(node);
  return () => observer.disconnect();
}
