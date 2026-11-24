import { isNewerNotificationSnapshot, NotificationClient } from "./client.ts";
import type { NotificationSubscription } from "./client.ts";
import { NOTIFICATION_PROTOCOL_VERSION } from "./generated/protocol.ts";
import type {
  NotificationActionProjection,
  NotificationId,
  NotificationMutationResult,
  NotificationRecordProjection,
  NotificationRejection,
  NotificationRequestId,
  NotificationSnapshot,
} from "./generated/protocol.ts";
import { notificationSeverityTone } from "./tone.ts";
import type { NotificationStatusTone } from "./tone.ts";
import { toastAction, toastTitle } from "./toast.ts";

import type { NotificationPort } from "./ports.ts";

export type NotificationControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };
export type NotificationPaginationStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "failed"; readonly error: unknown };
export type NotificationPendingKind = "markSeen" | "dismiss" | "action";

export interface NotificationPendingCommand {
  readonly requestId: NotificationRequestId;
  readonly notificationId: NotificationId;
  readonly kind: NotificationPendingKind;
}

export interface NotificationCommandRejection extends NotificationPendingCommand {
  readonly rejection: NotificationRejection;
}
export interface NotificationCommandFailure extends NotificationPendingCommand {
  readonly error: unknown;
}

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
  readonly action?: NotificationActionProjection | undefined;
}

export interface NotificationControllerOptions {
  readonly port: NotificationPort;
  readonly pageSize?: number;
  readonly toast?: { readonly select: (record: NotificationRecordProjection) => boolean };
  readonly actions?: NotificationActionExecutor;
}

export class NotificationController {
  readonly #client: NotificationClient;
  readonly #observers = new Set<() => void>();
  readonly #pending = new Map<NotificationRequestId, NotificationPendingCommand>();
  readonly #pageSize: number;
  readonly #toastSelector: ((record: NotificationRecordProjection) => boolean) | undefined;
  readonly #actionExecutor: NotificationActionExecutor | undefined;
  #status: NotificationControllerStatus = { kind: "idle" };
  #pagination: NotificationPaginationStatus = { kind: "idle" };
  #snapshot: NotificationSnapshot | undefined;
  #records: Array<NotificationRecordProjection> = [];
  #toasts: Array<NotificationToastProjection> = [];
  #selectedNotificationId: NotificationId | undefined;
  #commandRejection: NotificationCommandRejection | undefined;
  #commandFailure: NotificationCommandFailure | undefined;
  #subscription: NotificationSubscription | undefined;
  #started = false;
  #lifecycleRevision = 0;
  #toastSequence = 0;
  #pageLoad: Promise<void> | undefined;

  constructor(options: NotificationControllerOptions) {
    this.#client = new NotificationClient(options.port);
    this.#pageSize = options.pageSize ?? 100;
    this.#toastSelector = options.toast?.select;
    this.#actionExecutor = options.actions;
  }

  get status(): NotificationControllerStatus {
    return { ...this.#status };
  }
  get pagination(): NotificationPaginationStatus {
    return { ...this.#pagination };
  }
  get snapshot(): NotificationSnapshot | undefined {
    return structuredClone(this.#snapshot);
  }
  get records(): ReadonlyArray<NotificationRecordProjection> {
    return structuredClone(this.#records);
  }
  get toasts(): ReadonlyArray<NotificationToastProjection> {
    return structuredClone(this.#toasts);
  }
  get selectedNotificationId(): NotificationId | undefined {
    return this.#selectedNotificationId;
  }
  get selected(): NotificationRecordProjection | undefined {
    return structuredClone(
      this.#records.find((record) => record.notificationId === this.#selectedNotificationId),
    );
  }
  get commandRejection(): NotificationCommandRejection | undefined {
    return structuredClone(this.#commandRejection);
  }
  get commandFailure(): NotificationCommandFailure | undefined {
    return this.#commandFailure === undefined ? undefined : { ...this.#commandFailure };
  }
  get hasMore(): boolean {
    return this.#snapshot?.page.hasMore ?? false;
  }
  get pendingCommands(): ReadonlyArray<NotificationPendingCommand> {
    return Array.from(this.#pending.values(), (pending) => ({ ...pending }));
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  async start(): Promise<void> {
    if (this.#started) {
      return;
    }
    this.#started = true;
    const lifecycle = ++this.#lifecycleRevision;
    this.#setStatus({ kind: "loading" });
    const subscription = this.#client.subscribe(
      (snapshot) => {
        if (this.#isCurrent(lifecycle)) {
          this.#install(snapshot);
        }
      },
      ({ error }) => {
        if (this.#isCurrent(lifecycle)) {
          this.#setStatus({ kind: "failed", error });
        }
      },
      this.#pageSize,
    );
    this.#subscription = subscription;
    try {
      await subscription.ready;
      if (this.#isCurrent(lifecycle)) {
        this.#setStatus({ kind: "ready" });
      }
    } catch (error) {
      if (this.#isCurrent(lifecycle)) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  async stop(): Promise<void> {
    this.#started = false;
    this.#lifecycleRevision += 1;
    const subscription = this.#subscription;
    this.#subscription = undefined;
    this.#snapshot = undefined;
    this.#records = [];
    this.#toasts = [];
    this.#selectedNotificationId = undefined;
    this.#pending.clear();
    this.#pageLoad = undefined;
    this.#pagination = { kind: "idle" };
    this.#commandFailure = undefined;
    this.#commandRejection = undefined;
    this.#setStatus({ kind: "idle" });
    if (subscription !== undefined) {
      await subscription.dispose();
    }
  }

  select(notificationId: NotificationId | undefined): void {
    if (
      notificationId !== undefined &&
      !this.#records.some((record) => record.notificationId === notificationId)
    ) {
      throw new NotificationControllerUnknownRecordError(notificationId);
    }
    this.#selectedNotificationId = notificationId;
    this.#notify();
  }

  async loadMore(): Promise<void> {
    if (this.#pageLoad !== undefined) {
      return this.#pageLoad;
    }
    const current = this.#requireSnapshot();
    if (!current.page.hasMore) {
      return;
    }
    const lifecycle = this.#lifecycleRevision;
    const load = this.#applyPage(current, this.#records.length, lifecycle);
    this.#pageLoad = load;
    this.#setPagination({ kind: "loading" });
    try {
      await load;
      if (this.#pageLoad === load) {
        this.#setPagination({ kind: "idle" });
      }
    } catch (error) {
      // A load that a newer snapshot or a stop has abandoned no longer owns status.
      if (this.#pageLoad === load) {
        this.#setPagination({ kind: "failed", error });
      }
      throw error;
    } finally {
      if (this.#pageLoad === load) {
        this.#pageLoad = undefined;
      }
    }
  }

  markSeen(notificationId: NotificationId): Promise<NotificationMutationResult> {
    return this.#mutateRecord(notificationId, "markSeen");
  }
  dismiss(notificationId: NotificationId): Promise<NotificationMutationResult> {
    return this.#mutateRecord(notificationId, "dismiss");
  }

  dismissToast(id: string): void {
    const next = this.#toasts.filter((toast) => toast.id !== id);
    if (next.length === this.#toasts.length) {
      return;
    }
    this.#toasts = next;
    this.#notify();
  }

  async invokeAction(notificationId: NotificationId, referenceId: string): Promise<void> {
    const record = this.#requireRecord(notificationId);
    const action = record.draft.actions.find((candidate) => candidate.referenceId === referenceId);
    if (action === undefined) {
      throw new NotificationControllerUnknownActionError(notificationId, referenceId);
    }
    if (this.#actionExecutor === undefined) {
      throw new NotificationControllerActionUnavailableError();
    }
    const pending = {
      requestId: this.#client.nextRequestId(),
      notificationId,
      kind: "action" as const,
    };
    await this.#runPending(pending, async () => {
      await this.#actionExecutor?.admitAndExecute({
        notificationId,
        sourceId: record.draft.sourceId,
        referenceId: action.referenceId,
      });
    });
  }

  isPending(notificationId: NotificationId, kind: NotificationPendingKind): boolean {
    return [...this.#pending.values()].some(
      (item) => item.notificationId === notificationId && item.kind === kind,
    );
  }

  async #mutateRecord(
    notificationId: NotificationId,
    kind: "markSeen" | "dismiss",
  ): Promise<NotificationMutationResult> {
    const snapshot = this.#requireSnapshot();
    this.#requireRecord(notificationId);
    const command = {
      kind,
      requestId: this.#client.nextRequestId(),
      protocolVersion: NOTIFICATION_PROTOCOL_VERSION,
      authority: snapshot.authority,
      expectedLedgerRevision: snapshot.ledgerRevision,
      notificationId,
    } as const;
    return this.#runPending(
      { requestId: command.requestId, notificationId, kind },
      async (lifecycle) => {
        const result = await this.#client.mutate(command);
        if (!this.#isCurrent(lifecycle)) {
          return result;
        }
        if (result.status === "rejected") {
          this.#commandRejection = {
            requestId: command.requestId,
            notificationId,
            kind,
            rejection: structuredClone(result.rejection),
          };
        }
        this.#install(result.snapshot);
        return result;
      },
    );
  }

  async #runPending<Result>(
    pending: NotificationPendingCommand,
    run: (lifecycle: number) => Promise<Result>,
  ): Promise<Result> {
    if (this.isPending(pending.notificationId, pending.kind)) {
      throw new NotificationControllerCommandPendingError(pending.notificationId, pending.kind);
    }
    const lifecycle = this.#lifecycleRevision;
    this.#pending.set(pending.requestId, pending);
    this.#commandFailure = undefined;
    this.#commandRejection = undefined;
    this.#notify();
    try {
      return await run(lifecycle);
    } catch (error) {
      if (this.#isCurrent(lifecycle)) {
        this.#commandFailure = { ...pending, error };
      }
      throw error;
    } finally {
      if (this.#isCurrent(lifecycle)) {
        this.#pending.delete(pending.requestId);
        this.#notify();
      }
    }
  }

  /**
   * Applies one requested page only while the live state is still the basis it
   * was asked against. A page is a slice, so when the ledger has moved past the
   * basis it resyncs page zero instead of installing the slice as state.
   */
  async #applyPage(basis: NotificationSnapshot, offset: number, lifecycle: number): Promise<void> {
    const response = (await this.#client.snapshot(offset, this.#pageSize)).snapshot;
    const live = this.#pageFence(basis, offset, lifecycle);
    if (live === undefined) {
      return;
    }
    if (!isSameLedgerPosition(response, live)) {
      await this.#resyncFirstPage(lifecycle);
      return;
    }
    const known = new Set(this.#records.map((record) => record.notificationId));
    this.#records = [
      ...this.#records,
      ...structuredClone(
        response.page.records.filter((record) => !known.has(record.notificationId)),
      ),
    ];
    this.#snapshot = {
      ...live,
      page: { ...live.page, hasMore: response.page.hasMore, records: [...this.#records] },
    };
    this.#notify();
  }

  /** The live snapshot, when a page requested at `offset` against `basis` may still apply. */
  #pageFence(
    basis: NotificationSnapshot,
    offset: number,
    lifecycle: number,
  ): NotificationSnapshot | undefined {
    const live = this.#snapshot;
    if (!this.#isCurrent(lifecycle) || live === undefined || this.#records.length !== offset) {
      return undefined;
    }
    return isSameLedgerPosition(live, basis) ? live : undefined;
  }

  async #resyncFirstPage(lifecycle: number): Promise<void> {
    const first = (await this.#client.snapshot(0, this.#pageSize)).snapshot;
    if (this.#isCurrent(lifecycle)) {
      this.#install(first);
    }
  }

  #setPagination(pagination: NotificationPaginationStatus): void {
    this.#pagination = pagination;
    this.#notify();
  }

  #install(incoming: NotificationSnapshot): void {
    if (!isNewerNotificationSnapshot(incoming, this.#snapshot)) {
      return;
    }
    const candidate = structuredClone(incoming);
    const previous = this.#snapshot;
    this.#snapshot = candidate;
    this.#records = [...candidate.page.records];
    // A newer snapshot abandons any page request it was asked against, and clearing
    // the load withdraws that request's right to set status. A failure recorded
    // against the older state must not outlive the snapshot, even with no further pages.
    this.#pageLoad = undefined;
    this.#pagination = { kind: "idle" };
    if (previous !== undefined && this.#toastSelector !== undefined) {
      const existing = new Set(this.#toasts.map((toast) => toast.notificationId));
      const fresh = candidate.page.records.filter(
        (record) =>
          record.readState === "unseen" &&
          record.lastChangedLedgerRevision > previous.ledgerRevision &&
          this.#toastSelector?.(structuredClone(record)) === true &&
          !existing.has(record.notificationId),
      );
      this.#toasts = [...this.#toasts, ...fresh.map((record) => this.#projectToast(record))];
    }
    const retained = new Set(this.#records.map((record) => record.notificationId));
    this.#toasts = this.#toasts.filter((toast) => retained.has(toast.notificationId));
    if (this.#selectedNotificationId !== undefined && !retained.has(this.#selectedNotificationId)) {
      this.#selectedNotificationId = undefined;
    }
    this.#notify();
  }

  #projectToast(record: NotificationRecordProjection): NotificationToastProjection {
    return {
      id: `notification-toast:${++this.#toastSequence}`,
      notificationId: record.notificationId,
      title: toastTitle(record.draft),
      description: record.draft.summary,
      tone: notificationSeverityTone(record.draft.severity),
      action: toastAction(record.draft),
    };
  }

  #requireSnapshot(): NotificationSnapshot {
    if (!this.#started || this.#snapshot === undefined) {
      throw new NotificationControllerUnavailableError();
    }
    return this.#snapshot;
  }
  #requireRecord(id: NotificationId): NotificationRecordProjection {
    const record = this.#records.find((candidate) => candidate.notificationId === id);
    if (record === undefined) {
      throw new NotificationControllerUnknownRecordError(id);
    }
    return record;
  }
  #isCurrent(revision: number): boolean {
    return this.#started && this.#lifecycleRevision === revision;
  }
  #setStatus(status: NotificationControllerStatus): void {
    this.#status = status;
    this.#notify();
  }
  #notify(): void {
    for (const observer of this.#observers) {
      observer();
    }
  }
}

function isSameLedgerPosition(a: NotificationSnapshot, b: NotificationSnapshot): boolean {
  return (
    a.authority.authorityId === b.authority.authorityId &&
    a.authority.authorityEpoch === b.authority.authorityEpoch &&
    a.ledgerRevision === b.ledgerRevision
  );
}

export class NotificationControllerUnavailableError extends Error {
  constructor() {
    super("notification controller is not ready");
    this.name = "NotificationControllerUnavailableError";
  }
}
export class NotificationControllerUnknownRecordError extends Error {
  constructor(readonly notificationId: string) {
    super(`unknown notification: ${notificationId}`);
    this.name = "NotificationControllerUnknownRecordError";
  }
}
export class NotificationControllerUnknownActionError extends Error {
  constructor(
    readonly notificationId: string,
    readonly referenceId: string,
  ) {
    super(`unknown notification action ${referenceId} on ${notificationId}`);
    this.name = "NotificationControllerUnknownActionError";
  }
}
export class NotificationControllerActionUnavailableError extends Error {
  constructor() {
    super("notification action executor is not configured");
    this.name = "NotificationControllerActionUnavailableError";
  }
}
export class NotificationControllerCommandPendingError extends Error {
  constructor(
    readonly notificationId: string,
    readonly kind: NotificationPendingKind,
  ) {
    super(`notification ${kind} already pending for ${notificationId}`);
    this.name = "NotificationControllerCommandPendingError";
  }
}
