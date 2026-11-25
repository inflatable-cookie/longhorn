import { NotificationController } from "@inflatable-cookie/longhorn/notifications";
import type {
  NotificationControllerOptions,
  NotificationPendingKind,
} from "@inflatable-cookie/longhorn/notifications";
import type {
  NotificationId,
  NotificationMutationResult,
} from "@inflatable-cookie/longhorn/notifications/protocol";

import { freezeProjection } from "../../bindings/freeze-projection.ts";

export class NotificationSession {
  readonly #controller: NotificationController;
  readonly #unobserve: () => void;
  #projection: ReturnType<typeof project>;

  constructor(options: NotificationControllerOptions) {
    this.#controller = new NotificationController(options);
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

  get records() {
    return this.#projection.records;
  }

  get toasts() {
    return this.#projection.toasts;
  }

  get selectedNotificationId() {
    return this.#projection.selectedNotificationId;
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

  get pagination() {
    return this.#projection.pagination;
  }

  get hasMore() {
    return this.#projection.hasMore;
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
  observe(observer: () => void): () => void {
    return this.#controller.observe(observer);
  }
  select(notificationId: NotificationId | undefined): void {
    this.#controller.select(notificationId);
  }
  loadMore(): Promise<void> {
    return this.#controller.loadMore();
  }
  markSeen(notificationId: NotificationId): Promise<NotificationMutationResult> {
    return this.#controller.markSeen(notificationId);
  }
  dismiss(notificationId: NotificationId): Promise<NotificationMutationResult> {
    return this.#controller.dismiss(notificationId);
  }
  dismissToast(id: string): void {
    this.#controller.dismissToast(id);
  }
  invokeAction(notificationId: NotificationId, referenceId: string): Promise<void> {
    return this.#controller.invokeAction(notificationId, referenceId);
  }
  isPending(notificationId: NotificationId, kind: NotificationPendingKind): boolean {
    return this.#controller.isPending(notificationId, kind);
  }
}

function project(controller: NotificationController) {
  const failure = controller.commandFailure;
  return {
    status: Object.freeze({ ...controller.status }),
    snapshot: freezeProjection(controller.snapshot),
    records: freezeProjection(controller.records),
    toasts: freezeProjection(controller.toasts),
    selectedNotificationId: controller.selectedNotificationId,
    selected: freezeProjection(controller.selected),
    pendingCommands: freezeProjection(controller.pendingCommands),
    commandRejection: freezeProjection(controller.commandRejection),
    commandFailure: failure === undefined ? undefined : Object.freeze({ ...failure }),
    pagination: Object.freeze({ ...controller.pagination }),
    hasMore: controller.hasMore,
  };
}
