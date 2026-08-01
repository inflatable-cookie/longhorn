import {
  OperationController,
  type OperationCommandFailure,
  type OperationCommandRejection,
  type OperationControllerOptions,
  type OperationControllerStatus,
  type OperationPendingCommand,
} from "../controller.ts";
import type {
  OperationCancellationResult,
  OperationEntryProjection,
  OperationId,
  OperationMutationResult,
  OperationSnapshot,
} from "../generated/protocol.ts";

export class OperationSession {
  readonly #controller: OperationController;
  readonly #unobserve: () => void;
  status = $state<OperationControllerStatus>({ kind: "idle" });
  snapshot = $state<OperationSnapshot | undefined>();
  active = $state<OperationEntryProjection[]>([]);
  recent = $state<OperationEntryProjection[]>([]);
  selectedOperationId = $state<OperationId | undefined>();
  selected = $state<OperationEntryProjection | undefined>();
  pendingCommands = $state<OperationPendingCommand[]>([]);
  commandRejection = $state<OperationCommandRejection | undefined>();
  commandFailure = $state<OperationCommandFailure | undefined>();

  constructor(options: OperationControllerOptions) {
    this.#controller = new OperationController(options);
    this.#unobserve = this.#controller.observe(() => this.#sync());
    this.#sync();
  }

  start(): Promise<void> {
    return this.#controller.start();
  }

  stop(): Promise<void> {
    return this.#controller.stop();
  }

  async dispose(): Promise<void> {
    this.#unobserve();
    await this.#controller.stop();
  }

  select(operationId: OperationId | undefined): void {
    this.#controller.select(operationId);
  }

  cancel(operationId: OperationId): Promise<OperationCancellationResult> {
    return this.#controller.cancel(operationId);
  }

  dismiss(operationId: OperationId): Promise<OperationMutationResult> {
    return this.#controller.dismiss(operationId);
  }

  isCancellationPending(operationId: OperationId): boolean {
    return this.#controller.isCancellationPending(operationId);
  }

  isDismissalPending(operationId: OperationId): boolean {
    return this.#controller.isDismissalPending(operationId);
  }

  #sync(): void {
    this.status = this.#controller.status;
    this.snapshot = this.#controller.snapshot;
    this.active = [...this.#controller.active];
    this.recent = [...this.#controller.recent];
    this.selectedOperationId = this.#controller.selectedOperationId;
    this.selected = this.#controller.selected;
    this.pendingCommands = [...this.#controller.pendingCommands];
    this.commandRejection = this.#controller.commandRejection;
    this.commandFailure = this.#controller.commandFailure;
  }
}
