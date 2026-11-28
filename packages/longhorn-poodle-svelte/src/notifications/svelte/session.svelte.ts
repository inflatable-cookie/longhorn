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

import { SessionProjection } from "../../bindings/session-projection.svelte.ts";

export class NotificationSession {
  readonly #controller: NotificationController;
  readonly #projection: SessionProjection<ReturnType<typeof project>>;

  constructor(options: NotificationControllerOptions) {
    this.#controller = new NotificationController(options);
    this.#projection = new SessionProjection(this.#controller, () => project(this.#controller));
  }

  get status() {
    return this.#projection.retryFailure ?? this.#projection.current.status;
  }

  get snapshot() {
    return this.#projection.current.snapshot;
  }

  get records() {
    return this.#projection.current.records;
  }

  get toasts() {
    return this.#projection.current.toasts;
  }

  get selectedNotificationId() {
    return this.#projection.current.selectedNotificationId;
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

  get pagination() {
    return this.#projection.current.pagination;
  }

  get hasMore() {
    return this.#projection.current.hasMore;
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
