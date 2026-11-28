import { OperationController } from "@inflatable-cookie/longhorn/operation";
import type { OperationControllerOptions } from "@inflatable-cookie/longhorn/operation";
import type {
  OperationCancellationResult,
  OperationId,
  OperationMutationResult,
} from "@inflatable-cookie/longhorn/operation/protocol";

import { freezeProjection } from "../../bindings/freeze-projection.ts";

import { SessionProjection } from "../../bindings/session-projection.svelte.ts";

export class OperationSession {
  readonly #controller: OperationController;
  readonly #projection: SessionProjection<ReturnType<typeof project>>;

  constructor(options: OperationControllerOptions) {
    this.#controller = new OperationController(options);
    this.#projection = new SessionProjection(this.#controller, () => project(this.#controller));
  }

  get status() {
    return this.#projection.retryFailure ?? this.#projection.current.status;
  }

  get snapshot() {
    return this.#projection.current.snapshot;
  }

  get active() {
    return this.#projection.current.active;
  }

  get recent() {
    return this.#projection.current.recent;
  }

  get selectedOperationId() {
    return this.#projection.current.selectedOperationId;
  }

  get selected() {
    return this.#projection.current.selected;
  }

  get pendingCommands() {
    return this.#projection.current.pendingCommands;
  }

  get commandRejection() {
    return this.#projection.current.commandRejection;
  }

  get commandFailure() {
    return this.#projection.current.commandFailure;
  }

  start(): Promise<void> {
    return this.#projection.start();
  }

  stop(): Promise<void> {
    return this.#projection.stop();
  }

  retry(): Promise<void> {
    const status = this.status;
    return status.kind === "failed" ? this.#projection.retry(status.error) : this.start();
  }

  async dispose(): Promise<void> {
    await this.#projection.dispose();
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
