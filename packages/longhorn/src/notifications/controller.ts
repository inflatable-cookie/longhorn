import { isNewerNotificationSnapshot, NotificationClient, type NotificationSubscription } from "./client.ts";
import {
  NOTIFICATION_PROTOCOL_VERSION,
  type NotificationActionProjection,
  type NotificationId,
  type NotificationMutationResult,
  type NotificationRecordProjection,
  type NotificationRejection,
  type NotificationRequestId,
  type NotificationSnapshot,
} from "./generated/protocol.ts";
import { notificationSeverityTone, type NotificationStatusTone } from "./tone.ts";
import { toastAction, toastTitle } from "./toast.ts";

import type { NotificationPort } from "./ports.ts";

export type NotificationControllerStatus = { readonly kind: "idle" } | { readonly kind: "loading" } | { readonly kind: "ready" } | { readonly kind: "failed"; readonly error: unknown };
export type NotificationPendingKind = "markSeen" | "dismiss" | "action";

export interface NotificationPendingCommand {
  readonly requestId: NotificationRequestId;
  readonly notificationId: NotificationId;
  readonly kind: NotificationPendingKind;
}

export interface NotificationCommandRejection extends NotificationPendingCommand { readonly rejection: NotificationRejection; }
export interface NotificationCommandFailure extends NotificationPendingCommand { readonly error: unknown; }

export interface NotificationActionInvocation {
  readonly notificationId: NotificationId;
  readonly sourceId: string;
  readonly referenceId: string;
}

export interface NotificationActionExecutor {
  admitAndExecute(invocation: NotificationActionInvocation): Promise<void>;
}

export interface NotificationToastProjection {
  readonly id: string;
  readonly notificationId: NotificationId;
  readonly title: string;
  readonly description: string;
  readonly tone: NotificationStatusTone;
  readonly action?: NotificationActionProjection;
}

export interface NotificationControllerOptions {
  readonly port: NotificationPort;
  readonly pageSize?: number;
  readonly toast?: { readonly select: (record: NotificationRecordProjection) => boolean };
  readonly actions?: NotificationActionExecutor;
}

export class NotificationController {
  readonly client: NotificationClient;
  readonly observers = new Set<() => void>();
  readonly pending = new Map<NotificationRequestId, NotificationPendingCommand>();
  readonly pageSize: number;
  readonly toastSelector: ((record: NotificationRecordProjection) => boolean) | undefined;
  readonly actionExecutor: NotificationActionExecutor | undefined;
  status: NotificationControllerStatus = { kind: "idle" };
  snapshot: NotificationSnapshot | undefined;
  records: NotificationRecordProjection[] = [];
  toasts: NotificationToastProjection[] = [];
  selectedNotificationId: NotificationId | undefined;
  commandRejection: NotificationCommandRejection | undefined;
  commandFailure: NotificationCommandFailure | undefined;
  subscription: NotificationSubscription | undefined;
  started = false;
  lifecycleRevision = 0;
  toastSequence = 0;

  constructor(options: NotificationControllerOptions) {
    this.client = new NotificationClient(options.port);
    this.pageSize = options.pageSize ?? 100;
    this.toastSelector = options.toast?.select;
    this.actionExecutor = options.actions;
  }

  get selected(): NotificationRecordProjection | undefined { return this.records.find((record) => record.notificationId === this.selectedNotificationId); }
  get hasMore(): boolean { return this.snapshot?.page.hasMore ?? false; }
  get pendingCommands(): readonly NotificationPendingCommand[] { return [...this.pending.values()]; }

  observe(observer: () => void): () => void { this.observers.add(observer); return () => this.observers.delete(observer); }

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    const lifecycle = ++this.lifecycleRevision;
    this.setStatus({ kind: "loading" });
    const subscription = this.client.subscribe(
      (snapshot) => { if (this.isCurrent(lifecycle)) this.install(snapshot); },
      ({ error }) => { if (this.isCurrent(lifecycle)) this.setStatus({ kind: "failed", error }); },
      this.pageSize,
    );
    this.subscription = subscription;
    try {
      await subscription.ready;
      if (this.isCurrent(lifecycle)) this.setStatus({ kind: "ready" });
    } catch (error) {
      if (this.isCurrent(lifecycle)) this.setStatus({ kind: "failed", error });
    }
  }

  async stop(): Promise<void> {
    this.started = false;
    this.lifecycleRevision += 1;
    const subscription = this.subscription;
    this.subscription = undefined;
    this.snapshot = undefined;
    this.records = [];
    this.toasts = [];
    this.selectedNotificationId = undefined;
    this.pending.clear();
    this.commandFailure = undefined;
    this.commandRejection = undefined;
    this.setStatus({ kind: "idle" });
    if (subscription !== undefined) await subscription.dispose();
  }

  select(notificationId: NotificationId | undefined): void {
    if (notificationId !== undefined && !this.records.some((record) => record.notificationId === notificationId)) throw new NotificationControllerUnknownRecordError(notificationId);
    this.selectedNotificationId = notificationId;
    this.notify();
  }

  async loadMore(): Promise<void> {
    const current = this.requireSnapshot();
    if (!current.page.hasMore) return;
    const next = (await this.client.snapshot(this.records.length, this.pageSize)).snapshot;
    if (next.authority.authorityId !== current.authority.authorityId || next.authority.authorityEpoch !== current.authority.authorityEpoch || next.ledgerRevision !== current.ledgerRevision) {
      this.install(next);
      return;
    }
    const known = new Set(this.records.map((record) => record.notificationId));
    this.records = [...this.records, ...next.page.records.filter((record) => !known.has(record.notificationId))];
    this.snapshot = { ...current, page: { ...current.page, hasMore: next.page.hasMore, records: [...this.records] } };
    this.notify();
  }

  markSeen(notificationId: NotificationId): Promise<NotificationMutationResult> { return this.mutateRecord(notificationId, "markSeen"); }
  dismiss(notificationId: NotificationId): Promise<NotificationMutationResult> { return this.mutateRecord(notificationId, "dismiss"); }

  dismissToast(id: string): void {
    const next = this.toasts.filter((toast) => toast.id !== id);
    if (next.length === this.toasts.length) return;
    this.toasts = next;
    this.notify();
  }

  async invokeAction(notificationId: NotificationId, referenceId: string): Promise<void> {
    const record = this.requireRecord(notificationId);
    const action = record.draft.actions.find((candidate) => candidate.referenceId === referenceId);
    if (action === undefined) throw new NotificationControllerUnknownActionError(notificationId, referenceId);
    if (this.actionExecutor === undefined) throw new NotificationControllerActionUnavailableError();
    const pending = { requestId: this.client.nextRequestId(), notificationId, kind: "action" as const };
    await this.runPending(pending, async () => {
      await this.actionExecutor?.admitAndExecute({ notificationId, sourceId: record.draft.sourceId, referenceId: action.referenceId });
    });
  }

  isPending(notificationId: NotificationId, kind: NotificationPendingKind): boolean {
    return [...this.pending.values()].some((item) => item.notificationId === notificationId && item.kind === kind);
  }

  private async mutateRecord(notificationId: NotificationId, kind: "markSeen" | "dismiss"): Promise<NotificationMutationResult> {
    const snapshot = this.requireSnapshot();
    this.requireRecord(notificationId);
    const command = {
      kind,
      requestId: this.client.nextRequestId(),
      protocolVersion: NOTIFICATION_PROTOCOL_VERSION,
      authority: snapshot.authority,
      expectedLedgerRevision: snapshot.ledgerRevision,
      notificationId,
    } as const;
    return this.runPending({ requestId: command.requestId, notificationId, kind }, async () => {
      const result = await this.client.mutate(command);
      this.install(result.snapshot);
      if (result.status === "rejected") this.commandRejection = { requestId: command.requestId, notificationId, kind, rejection: result.rejection };
      return result;
    });
  }

  private async runPending<Result>(pending: NotificationPendingCommand, run: () => Promise<Result>): Promise<Result> {
    if (this.isPending(pending.notificationId, pending.kind)) throw new NotificationControllerCommandPendingError(pending.notificationId, pending.kind);
    const lifecycle = this.lifecycleRevision;
    this.pending.set(pending.requestId, pending);
    this.commandFailure = undefined;
    this.commandRejection = undefined;
    this.notify();
    try { return await run(); }
    catch (error) {
      if (this.isCurrent(lifecycle)) this.commandFailure = { ...pending, error };
      throw error;
    } finally {
      if (this.isCurrent(lifecycle)) { this.pending.delete(pending.requestId); this.notify(); }
    }
  }

  private install(candidate: NotificationSnapshot): void {
    if (!isNewerNotificationSnapshot(candidate, this.snapshot)) return;
    const previous = this.snapshot;
    this.snapshot = candidate;
    this.records = [...candidate.page.records];
    if (previous !== undefined && this.toastSelector !== undefined) {
      const existing = new Set(this.toasts.map((toast) => toast.notificationId));
      const fresh = candidate.page.records.filter((record) => record.readState === "unseen" && record.lastChangedLedgerRevision > previous.ledgerRevision && this.toastSelector?.(record) === true && !existing.has(record.notificationId));
      this.toasts = [...this.toasts, ...fresh.map((record) => this.projectToast(record))];
    }
    const retained = new Set(this.records.map((record) => record.notificationId));
    this.toasts = this.toasts.filter((toast) => retained.has(toast.notificationId));
    if (this.selectedNotificationId !== undefined && !retained.has(this.selectedNotificationId)) this.selectedNotificationId = undefined;
    this.notify();
  }

  private projectToast(record: NotificationRecordProjection): NotificationToastProjection {
    return {
      id: `notification-toast:${++this.toastSequence}`,
      notificationId: record.notificationId,
      title: toastTitle(record.draft),
      description: record.draft.summary,
      tone: notificationSeverityTone(record.draft.severity),
      action: toastAction(record.draft),
    };
  }

  private requireSnapshot(): NotificationSnapshot { if (!this.started || this.snapshot === undefined) throw new NotificationControllerUnavailableError(); return this.snapshot; }
  private requireRecord(id: NotificationId): NotificationRecordProjection { const record = this.records.find((candidate) => candidate.notificationId === id); if (record === undefined) throw new NotificationControllerUnknownRecordError(id); return record; }
  private isCurrent(revision: number): boolean { return this.started && this.lifecycleRevision === revision; }
  private setStatus(status: NotificationControllerStatus): void { this.status = status; this.notify(); }
  private notify(): void { for (const observer of this.observers) observer(); }
}

export class NotificationControllerUnavailableError extends Error { constructor() { super("notification controller is not ready"); this.name = "NotificationControllerUnavailableError"; } }
export class NotificationControllerUnknownRecordError extends Error { constructor(readonly notificationId: string) { super(`unknown notification: ${notificationId}`); this.name = "NotificationControllerUnknownRecordError"; } }
export class NotificationControllerUnknownActionError extends Error { constructor(readonly notificationId: string, readonly referenceId: string) { super(`unknown notification action ${referenceId} on ${notificationId}`); this.name = "NotificationControllerUnknownActionError"; } }
export class NotificationControllerActionUnavailableError extends Error { constructor() { super("notification action executor is not configured"); this.name = "NotificationControllerActionUnavailableError"; } }
export class NotificationControllerCommandPendingError extends Error { constructor(readonly notificationId: string, readonly kind: NotificationPendingKind) { super(`notification ${kind} already pending for ${notificationId}`); this.name = "NotificationControllerCommandPendingError"; } }
