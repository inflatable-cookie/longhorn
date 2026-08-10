import { HistoryClient } from "./client.ts";
import {
  HISTORY_PROTOCOL_VERSION,
  type HistoryChangedEvent,
  type HistoryEntryId,
  type HistoryEntryRecord,
  type HistoryNavigationRejectionProjection,
  type HistoryNavigationResult,
  type HistoryNavigationTargetProjection,
  type HistoryPageSnapshot,
  type HistorySnapshot,
} from "./generated/protocol.ts";
import { HISTORY_MAXIMUM_PROJECTION_PAGE_SIZE } from "./generated/protocol.ts";
import type { HistoryPort, HistoryUnlisten } from "./ports.ts";

export type HistoryControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };

export interface HistoryControllerOptions {
  readonly port: HistoryPort;
  readonly pageSize?: number;
}

export class HistoryController {
  readonly #client: HistoryClient;
  readonly #observers = new Set<() => void>();
  #status: HistoryControllerStatus = { kind: "idle" };
  #snapshot: HistorySnapshot | undefined;
  #pageSnapshot: HistoryPageSnapshot | undefined;
  #rejection: HistoryNavigationRejectionProjection | undefined;
  #filter = "";
  #page = 1;
  #pageSize: number;
  #navigationPending = false;
  #started = false;
  #lifecycleRevision = 0;
  #loadRevision = 0;
  #navigationRevision = 0;
  #unlisten: HistoryUnlisten[] = [];

  constructor(options: HistoryControllerOptions) {
    this.#client = new HistoryClient(options.port);
    this.#pageSize = checkedPageSize(options.pageSize ?? 50);
  }

  get status(): HistoryControllerStatus {
    return this.#status;
  }

  get snapshot(): HistorySnapshot | undefined {
    return this.#snapshot;
  }

  get pageSnapshot(): HistoryPageSnapshot | undefined {
    return this.#pageSnapshot;
  }

  get rejection() {
    return this.#rejection;
  }

  get filter(): string {
    return this.#filter;
  }

  get page(): number {
    return this.#page;
  }

  get pageSize(): number {
    return this.#pageSize;
  }

  get totalEntries(): number {
    return this.#pageSnapshot?.totalEntries ?? 0;
  }

  get entries(): readonly HistoryEntryRecord[] {
    const entries = this.#pageSnapshot?.entries ?? [];
    const query = this.#filter.trim().toLocaleLowerCase();
    if (query.length === 0) return entries;
    return entries.filter(
      (entry) =>
        entry.label.toLocaleLowerCase().includes(query) ||
        entry.kindId?.toLocaleLowerCase().includes(query) === true,
    );
  }

  get navigationPending(): boolean {
    return this.#navigationPending;
  }

  get canUndo(): boolean {
    return (this.#snapshot?.summary.undoDepth ?? 0) > 0;
  }

  get canRedo(): boolean {
    return (this.#snapshot?.summary.redoDepth ?? 0) > 0;
  }

  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  async start(): Promise<void> {
    if (this.#started) return;
    this.#started = true;
    const lifecycleRevision = ++this.#lifecycleRevision;
    this.#setStatus({ kind: "loading" });
    try {
      const unlisten = await this.#client.listen((event) => {
        this.#handleChanged(event);
      });
      if (!this.#isCurrentLifecycle(lifecycleRevision)) {
        await unlisten();
        return;
      }
      this.#unlisten.push(unlisten);
      await this.refresh();
    } catch (error) {
      if (this.#isCurrentLifecycle(lifecycleRevision)) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  async stop(): Promise<void> {
    this.#started = false;
    this.#lifecycleRevision += 1;
    this.#loadRevision += 1;
    this.#navigationRevision += 1;
    const unlisten = this.#unlisten.splice(0);
    await Promise.allSettled(unlisten.map((dispose) => dispose()));
    this.#snapshot = undefined;
    this.#pageSnapshot = undefined;
    this.#rejection = undefined;
    this.#filter = "";
    this.#page = 1;
    this.#navigationPending = false;
    this.#setStatus({ kind: "idle" });
  }

  async refresh(): Promise<void> {
    if (!this.#started) return;
    const revision = ++this.#loadRevision;
    try {
      const state = await this.#readAuthoritativeState();
      if (!this.#started || revision !== this.#loadRevision) return;
      this.#install(state.snapshot, state.page);
      this.#setStatus({ kind: "ready" });
    } catch (error) {
      if (this.#started && revision === this.#loadRevision) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  setFilter(filter: string): void {
    this.#filter = filter;
    this.#notify();
  }

  async setPage(page: number): Promise<void> {
    this.#page = checkedPage(page);
    await this.refresh();
  }

  async setPageSize(pageSize: number): Promise<void> {
    this.#pageSize = checkedPageSize(pageSize);
    this.#page = 1;
    await this.refresh();
  }

  undo(): Promise<HistoryNavigationResult> {
    return this.#navigate({ kind: "undo" });
  }

  redo(): Promise<HistoryNavigationResult> {
    return this.#navigate({ kind: "redo" });
  }

  checkout(entryId: HistoryEntryId): Promise<HistoryNavigationResult> {
    return this.#navigate({ kind: "checkout", entryId });
  }

  async #navigate(
    target: HistoryNavigationTargetProjection,
  ): Promise<HistoryNavigationResult> {
    const snapshot = this.#snapshot;
    if (!this.#started || snapshot === undefined) {
      throw new HistoryControllerUnavailableError();
    }
    const revision = ++this.#navigationRevision;
    this.#navigationPending = true;
    this.#rejection = undefined;
    this.#notify();
    try {
      const result = await this.#client.navigate({
        protocolVersion: HISTORY_PROTOCOL_VERSION,
        authorityEpoch: snapshot.authorityEpoch,
        historyId: snapshot.summary.historyId,
        planId: this.#client.nextPlanId(),
        expectedRevision: snapshot.summary.revision,
        target,
      });
      if (
        !this.#started ||
        revision !== this.#navigationRevision
      ) {
        throw new HistoryLateResultError();
      }
      this.#rejection =
        result.status === "rejected" ? result.rejection : undefined;
      const page = await this.#readPageForSnapshot(result.snapshot);
      if (
        !this.#started ||
        revision !== this.#navigationRevision
      ) {
        throw new HistoryLateResultError();
      }
      this.#install(result.snapshot, page);
      this.#setStatus({ kind: "ready" });
      return result;
    } catch (error) {
      if (
        this.#started &&
        revision === this.#navigationRevision &&
        !(error instanceof HistoryLateResultError)
      ) {
        this.#setStatus({ kind: "failed", error });
      }
      throw error;
    } finally {
      if (revision === this.#navigationRevision) {
        this.#navigationPending = false;
        this.#notify();
      }
    }
  }

  async #readAuthoritativeState(): Promise<{
    snapshot: HistorySnapshot;
    page: HistoryPageSnapshot;
  }> {
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const snapshot = await this.#client.snapshot();
      const page = await this.#readPageForSnapshot(snapshot);
      if (sameAuthority(snapshot, page)) return { snapshot, page };
    }
    throw new HistoryProjectionGapError();
  }

  async #readPageForSnapshot(
    snapshot: HistorySnapshot,
  ): Promise<HistoryPageSnapshot> {
    const maximumPage = Math.max(
      1,
      Math.ceil(snapshot.summary.retainedEntryCount / this.#pageSize),
    );
    this.#page = Math.min(this.#page, maximumPage);
    return this.#client.page({
      protocolVersion: HISTORY_PROTOCOL_VERSION,
      authorityEpoch: snapshot.authorityEpoch,
      historyId: snapshot.summary.historyId,
      expectedRevision: snapshot.summary.revision,
      offset: (this.#page - 1) * this.#pageSize,
      limit: this.#pageSize,
    });
  }

  #install(snapshot: HistorySnapshot, page: HistoryPageSnapshot): void {
    if (!sameAuthority(snapshot, page)) throw new HistoryProjectionGapError();
    const current = this.#snapshot;
    if (
      current !== undefined &&
      snapshot.authorityEpoch === current.authorityEpoch &&
      snapshot.summary.historyId === current.summary.historyId &&
      snapshot.summary.revision < current.summary.revision
    ) {
      return;
    }
    this.#snapshot = snapshot;
    this.#pageSnapshot = page;
    this.#notify();
  }

  #handleChanged(event: HistoryChangedEvent): void {
    if (!this.#started) return;
    const snapshot = this.#snapshot;
    if (snapshot === undefined) {
      void this.refresh();
      return;
    }
    if (
      event.authorityEpoch !== snapshot.authorityEpoch ||
      event.historyId !== snapshot.summary.historyId
    ) {
      void this.refresh();
      return;
    }
    if (event.committedRevision <= snapshot.summary.revision) return;
    void this.refresh();
  }

  #isCurrentLifecycle(revision: number): boolean {
    return this.#started && revision === this.#lifecycleRevision;
  }

  #setStatus(status: HistoryControllerStatus): void {
    this.#status = status;
    this.#notify();
  }

  #notify(): void {
    for (const observer of this.#observers) observer();
  }
}

export class HistoryControllerUnavailableError extends Error {
  constructor() {
    super("history controller has no live authoritative snapshot");
    this.name = "HistoryControllerUnavailableError";
  }
}

export class HistoryProjectionGapError extends Error {
  constructor() {
    super("history page does not match the authoritative snapshot");
    this.name = "HistoryProjectionGapError";
  }
}

export class HistoryLateResultError extends Error {
  constructor() {
    super("history result arrived after its controller lifetime");
    this.name = "HistoryLateResultError";
  }
}

function sameAuthority(
  snapshot: HistorySnapshot,
  page: HistoryPageSnapshot,
): boolean {
  return (
    page.authorityEpoch === snapshot.authorityEpoch &&
    page.historyId === snapshot.summary.historyId &&
    page.revision === snapshot.summary.revision
  );
}

function checkedPage(value: number): number {
  if (!Number.isSafeInteger(value) || value < 1) {
    throw new RangeError("history page must be a positive safe integer");
  }
  return value;
}

function checkedPageSize(value: number): number {
  if (
    !Number.isSafeInteger(value) ||
    value < 1 ||
    value > HISTORY_MAXIMUM_PROJECTION_PAGE_SIZE
  ) {
    throw new RangeError(
      `history page size must be between 1 and ${HISTORY_MAXIMUM_PROJECTION_PAGE_SIZE}`,
    );
  }
  return value;
}
