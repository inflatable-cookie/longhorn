import { OperationController } from "@inflatable-cookie/longhorn/operation";
import type { OperationControllerOptions } from "@inflatable-cookie/longhorn/operation";
import type {
  OperationCancellationResult,
  OperationId,
  OperationMutationResult,
} from "@inflatable-cookie/longhorn/operation/protocol";

import { freezeProjection } from "../../bindings/freeze-projection.ts";

export class OperationSession {
  readonly #controller: OperationController;
  readonly #unobserve: () => void;
  #projection: ReturnType<typeof project>;

  constructor(options: OperationControllerOptions) {
    this.#controller = new OperationController(options);
    this.#projection = $state.raw(project(this.#controller));
    this.#unobserve = this.#controller.observe(() => {
      this.#projection = project(this.#controller);
    });
  }

  get status() {
    return this.#projection.status;
  }

  get snapshot() {
    return this.#projection.snapshot;
  }

  get active() {
    return this.#projection.active;
  }

  get recent() {
    return this.#projection.recent;
  }

  get selectedOperationId() {
    return this.#projection.selectedOperationId;
  }

  get selected() {
    return this.#projection.selected;
  }

  get pendingCommands() {
    return this.#projection.pendingCommands;
  }

  get commandRejection() {
    return this.#projection.commandRejection;
  }

  get commandFailure() {
    return this.#projection.commandFailure;
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
}

function project(controller: OperationController) {
  const failure = controller.commandFailure;
  return {
    status: Object.freeze({ ...controller.status }),
    snapshot: freezeProjection(structuredClone(controller.snapshot)),
    active: freezeProjection(structuredClone(controller.active)),
    recent: freezeProjection(structuredClone(controller.recent)),
    selectedOperationId: controller.selectedOperationId,
    selected: freezeProjection(structuredClone(controller.selected)),
    pendingCommands: freezeProjection(structuredClone(controller.pendingCommands)),
    commandRejection: freezeProjection(structuredClone(controller.commandRejection)),
    commandFailure: failure === undefined ? undefined : Object.freeze({ ...failure }),
  };
}
