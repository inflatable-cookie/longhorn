import {
  NotificationController,
  type NotificationCommandFailure,
  type NotificationCommandRejection,
  type NotificationControllerOptions,
  type NotificationControllerStatus,
  type NotificationPendingCommand,
  type NotificationPendingKind,
  type NotificationToastProjection,
} from "../controller.ts";
import type { NotificationId, NotificationMutationResult, NotificationRecordProjection, NotificationSnapshot } from "../generated/protocol.ts";

export class NotificationSession {
  readonly controller: NotificationController;
  readonly unobserve: () => void;
  status = $state<NotificationControllerStatus>({ kind: "idle" });
  snapshot = $state<NotificationSnapshot | undefined>();
  records = $state<NotificationRecordProjection[]>([]);
  toasts = $state<NotificationToastProjection[]>([]);
  selectedNotificationId = $state<NotificationId | undefined>();
  selected = $state<NotificationRecordProjection | undefined>();
  pendingCommands = $state<NotificationPendingCommand[]>([]);
  commandRejection = $state<NotificationCommandRejection | undefined>();
  commandFailure = $state<NotificationCommandFailure | undefined>();
  hasMore = $state(false);

  constructor(options: NotificationControllerOptions) {
    this.controller = new NotificationController(options);
    this.unobserve = this.controller.observe(() => this.sync());
    this.sync();
  }

  start(): Promise<void> { return this.controller.start(); }
  stop(): Promise<void> { return this.controller.stop(); }
  async dispose(): Promise<void> { this.unobserve(); await this.controller.stop(); }
  observe(observer: () => void): () => void { return this.controller.observe(observer); }
  select(notificationId: NotificationId | undefined): void { this.controller.select(notificationId); }
  loadMore(): Promise<void> { return this.controller.loadMore(); }
  markSeen(notificationId: NotificationId): Promise<NotificationMutationResult> { return this.controller.markSeen(notificationId); }
  dismiss(notificationId: NotificationId): Promise<NotificationMutationResult> { return this.controller.dismiss(notificationId); }
  dismissToast(id: string): void { this.controller.dismissToast(id); }
  invokeAction(notificationId: NotificationId, referenceId: string): Promise<void> { return this.controller.invokeAction(notificationId, referenceId); }
  isPending(notificationId: NotificationId, kind: NotificationPendingKind): boolean { return this.controller.isPending(notificationId, kind); }

  private sync(): void {
    this.status = this.controller.status;
    this.snapshot = this.controller.snapshot;
    this.records = [...this.controller.records];
    this.toasts = [...this.controller.toasts];
    this.selectedNotificationId = this.controller.selectedNotificationId;
    this.selected = this.controller.selected;
    this.pendingCommands = [...this.controller.pendingCommands];
    this.commandRejection = this.controller.commandRejection;
    this.commandFailure = this.controller.commandFailure;
    this.hasMore = this.controller.hasMore;
  }
}
