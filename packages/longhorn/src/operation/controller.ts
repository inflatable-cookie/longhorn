import { ControllerLifecycle } from "../core/controller-lifecycle.ts";
import { isNewerOperationSnapshot, OperationClient } from "./client.ts";
import { OPERATION_PROTOCOL_VERSION } from "./generated/protocol.ts";
import type {
  OperationCancellationResult,
  OperationEntryProjection,
  OperationId,
  OperationMutationResult,
  OperationRejection,
  OperationRequestId,
  OperationSnapshot,
} from "./generated/protocol.ts";
import type { OperationPort } from "./ports.ts";

export type OperationControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export type OperationPendingCommandKind = "cancellation" | "dismissal";

export interface OperationPendingCommand {
  readonly requestId: OperationRequestId;
  readonly operationId: OperationId;
  readonly kind: OperationPendingCommandKind;
}

export interface OperationCommandRejection extends OperationPendingCommand {
  readonly rejection: OperationRejection;
}

export interface OperationCommandFailure extends OperationPendingCommand {
  readonly error: unknown;
}

export interface OperationControllerOptions {
  readonly port: OperationPort;
}

export class OperationController {
  readonly #client: OperationClient;
  readonly #observers = new Set<() => void>();
  readonly #pending = new Map<OperationRequestId, OperationPendingCommand>();
  #status: OperationControllerStatus = { kind: "idle" };
  #snapshot: OperationSnapshot | undefined;
  #selectedOperationId: OperationId | undefined;
  #commandRejection: OperationCommandRejection | undefined;
  #commandFailure: OperationCommandFailure | undefined;
  readonly #lifecycle = new ControllerLifecycle();
  #commandRevision = 0;

  constructor(options: OperationControllerOptions) {
    this.#client = new OperationClient(options.port);
  }

  get status(): OperationControllerStatus {
    return this.#lifecycle.eventFailure ?? this.#status;
  }

  get snapshot(): OperationSnapshot | undefined {
    return this.#snapshot;
  }

  get active(): ReadonlyArray<OperationEntryProjection> {
    return this.#snapshot?.active ?? [];
  }

  get recent(): ReadonlyArray<OperationEntryProjection> {
    return this.#snapshot?.recent ?? [];
  }

  get selectedOperationId(): OperationId | undefined {
    return this.#selectedOperationId;
  }

  get selected(): OperationEntryProjection | undefined {
    if (this.#selectedOperationId === undefined) {
      return undefined;
    }
    return this.#entry(this.#selectedOperationId);
  }

  get pendingCommands(): ReadonlyArray<OperationPendingCommand> {
    return [...this.#pending.values()];
  }

  get commandRejection(): OperationCommandRejection | undefined {
    return this.#commandRejection;
  }

  get commandFailure(): OperationCommandFailure | undefined {
    return this.#commandFailure;
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  start(): Promise<void> {
    return this.#lifecycle.start({
      loading: () => this.#setStatus({ kind: "loading" }),
      connect: ({ isCurrent, reportFailure, watch }) => {
        const subscription = this.#client.subscribe((snapshot) => {
          if (isCurrent()) {
            this.#install(snapshot);
          }
        }, reportFailure);
        return watch(subscription, () => this.#setStatus({ kind: "ready" }));
      },
      failed: (error) => this.#setStatus({ kind: "failed", error }),
    });
  }

  stop(): Promise<void> {
    return this.#lifecycle.stop(() => {
      this.#commandRevision += 1;
      this.#snapshot = undefined;
      this.#selectedOperationId = undefined;
      this.#pending.clear();
      this.#commandRejection = undefined;
      this.#commandFailure = undefined;
      this.#setStatus({ kind: "idle" });
    });
  }

  select(operationId: OperationId | undefined): void {
    if (operationId !== undefined && this.#entry(operationId) === undefined) {
      throw new OperationControllerUnknownOperationError(operationId);
    }
    this.#selectedOperationId = operationId;
    this.#notify();
  }

  isCancellationPending(operationId: OperationId): boolean {
    return this.#hasPending(operationId, "cancellation");
  }

  isDismissalPending(operationId: OperationId): boolean {
    return this.#hasPending(operationId, "dismissal");
  }

  async cancel(operationId: OperationId): Promise<OperationCancellationResult> {
    const operation = this.#requireAvailableEntry(operationId);
    const command = {
      protocolVersion: OPERATION_PROTOCOL_VERSION,
      requestId: this.#client.nextRequestId(),
      authority: operation.authority,
      operationId,
      expectedOperationRevision: operation.revision,
    } as const;
    return this.#runCommand(
      { requestId: command.requestId, operationId, kind: "cancellation" },
      () => this.#client.cancel(command),
    );
  }

  async dismiss(operationId: OperationId): Promise<OperationMutationResult> {
    const operation = this.#requireAvailableEntry(operationId);
    const command = {
      kind: "dismiss",
      protocolVersion: OPERATION_PROTOCOL_VERSION,
      requestId: this.#client.nextRequestId(),
      authority: operation.authority,
      operationId,
      expectedOperationRevision: operation.revision,
    } as const;
    return this.#runCommand({ requestId: command.requestId, operationId, kind: "dismissal" }, () =>
      this.#client.mutate(command),
    );
  }

  async #runCommand<Result extends OperationCancellationResult | OperationMutationResult>(
    pending: OperationPendingCommand,
    run: () => Promise<Result>,
  ): Promise<Result> {
    if (this.#hasPending(pending.operationId, pending.kind)) {
      throw new OperationControllerCommandPendingError(pending.operationId, pending.kind);
    }
    const lifecycleRevision = this.#lifecycle.revision;
    const commandRevision = ++this.#commandRevision;
    this.#pending.set(pending.requestId, pending);
    this.#commandFailure = undefined;
    this.#commandRejection = undefined;
    this.#notify();

    try {
      const result = await run();
      if (this.#lifecycle.isCurrent(lifecycleRevision)) {
        this.#install(result.snapshot);
        if (commandRevision === this.#commandRevision && result.status === "rejected") {
          this.#commandRejection = { ...pending, rejection: result.rejection };
        }
      }
      return result;
    } catch (error) {
      if (
        this.#lifecycle.isCurrent(lifecycleRevision) &&
        commandRevision === this.#commandRevision
      ) {
        this.#commandFailure = { ...pending, error };
      }
      throw error;
    } finally {
      if (this.#lifecycle.isCurrent(lifecycleRevision)) {
        this.#pending.delete(pending.requestId);
        this.#notify();
      }
    }
  }

  #requireAvailableEntry(operationId: OperationId): OperationEntryProjection {
    if (!this.#lifecycle.active || this.#snapshot === undefined) {
      throw new OperationControllerUnavailableError();
    }
    const operation = this.#entry(operationId);
    if (operation === undefined) {
      throw new OperationControllerUnknownOperationError(operationId);
    }
    return operation;
  }

  #entry(operationId: OperationId): OperationEntryProjection | undefined {
    return [...this.active, ...this.recent].find((entry) => entry.operationId === operationId);
  }

  #hasPending(operationId: OperationId, kind: OperationPendingCommandKind): boolean {
    return [...this.#pending.values()].some(
      (pending) => pending.operationId === operationId && pending.kind === kind,
    );
  }

  #install(snapshot: OperationSnapshot): void {
    if (!isNewerOperationSnapshot(snapshot, this.#snapshot)) {
      return;
    }
    this.#snapshot = snapshot;
    if (
      this.#selectedOperationId !== undefined &&
      this.#entry(this.#selectedOperationId) === undefined
    ) {
      this.#selectedOperationId = undefined;
    }
    this.#notify();
  }

  #setStatus(status: OperationControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) {
      observer();
    }
  }
}

export class OperationControllerUnavailableError extends Error {
  constructor() {
    super("operation controller is not connected");
    this.name = "OperationControllerUnavailableError";
  }
}

export class OperationControllerUnknownOperationError extends Error {
  constructor(readonly operationId: OperationId) {
    super(`unknown operation: ${operationId}`);
    this.name = "OperationControllerUnknownOperationError";
  }
}

export class OperationControllerCommandPendingError extends Error {
  constructor(
    readonly operationId: OperationId,
    readonly kind: OperationPendingCommandKind,
  ) {
    super(`${kind} already pending for operation: ${operationId}`);
    this.name = "OperationControllerCommandPendingError";
  }
}
