import { ControllerLifecycle } from "../core/controller-lifecycle.ts";
import { ForkHistoryClient } from "./client.ts";
import {
  FORK_HISTORY_PROTOCOL_VERSION,
  MAXIMUM_FORK_HISTORY_PAGE_SIZE,
} from "./generated/protocol.ts";
import type {
  ForkBranchId,
  ForkBranchPageSnapshot,
  ForkChangedEvent,
  ForkContinuationPageSnapshot,
  ForkEntryRecord,
  ForkPruneResult,
  ForkDeleteResult,
  ForkNavigationResult,
  ForkNavigationTargetProjection,
  ForkPathPageSnapshot,
  ForkPathTargetProjection,
  ForkSnapshot,
} from "./generated/protocol.ts";
import type { ForkHistoryPort } from "./ports.ts";

export type ForkHistoryControllerStatus =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly error: unknown };
export interface ForkHistoryControllerOptions {
  readonly port: ForkHistoryPort;
  readonly pathPageSize?: number;
  readonly branchPageSize?: number;
  readonly continuationPageSize?: number;
}

export class ForkHistoryController {
  readonly #client: ForkHistoryClient;
  readonly #observers = new Set<() => void>();
  readonly #pathPageSize: number;
  readonly #branchPageSize: number;
  readonly #continuationPageSize: number;
  #status: ForkHistoryControllerStatus = { kind: "idle" };
  #snapshot: ForkSnapshot | undefined;
  #path: ForkPathPageSnapshot | undefined;
  #branches: ForkBranchPageSnapshot | undefined;
  #pathTarget: ForkPathTargetProjection = { kind: "default" };
  readonly #lifecycle = new ControllerLifecycle();
  #load = 0;
  #navigation = 0;
  #pending = false;

  constructor(options: ForkHistoryControllerOptions) {
    this.#client = new ForkHistoryClient(options.port);
    this.#pathPageSize = pageSize(options.pathPageSize ?? 50);
    this.#branchPageSize = pageSize(options.branchPageSize ?? 50);
    this.#continuationPageSize = pageSize(options.continuationPageSize ?? 50);
  }
  get status() {
    return this.#lifecycle.eventFailure ?? this.#status;
  }
  get snapshot() {
    return this.#snapshot;
  }
  get path() {
    return this.#path;
  }
  get branches() {
    return this.#branches;
  }
  get entries(): ReadonlyArray<ForkEntryRecord> {
    return this.#path?.entries ?? [];
  }
  get navigationPending() {
    return this.#pending;
  }
  get canUndo() {
    return (this.#snapshot?.summary.undoDepth ?? 0) > 0;
  }
  get canRedo() {
    return (this.#snapshot?.summary.redoDepth ?? 0) > 0;
  }
  observe(observer: () => void): () => void {
    this.#observers.add(observer);
    return () => this.#observers.delete(observer);
  }

  start(): Promise<void> {
    return this.#lifecycle.start({
      loading: () => this.#setStatus({ kind: "loading" }),
      connect: async ({ isCurrent, own, reportFailure }) => {
        const unlisten = await this.#client.listen((event) => {
          if (isCurrent()) {
            this.#changed(event);
          }
        }, reportFailure);
        await own(unlisten);
        if (isCurrent()) {
          await this.refresh();
        }
      },
      failed: (error) => this.#setStatus({ kind: "failed", error }),
    });
  }

  stop(): Promise<void> {
    return this.#lifecycle.stop(() => {
      this.#load += 1;
      this.#navigation += 1;
      this.#snapshot = undefined;
      this.#path = undefined;
      this.#branches = undefined;
      this.#pathTarget = { kind: "default" };
      this.#pending = false;
      this.#setStatus({ kind: "idle" });
    });
  }

  async refresh(): Promise<void> {
    if (!this.#lifecycle.active) {
      return;
    }
    const load = ++this.#load;
    // The offset a consumer had paged to, so a refresh returns the page they
    // were looking at rather than sending them back to the first one.
    const branchOffset = this.#branches?.offset;
    try {
      for (let attempt = 0; attempt < 2; attempt += 1) {
        const snapshot = await this.#client.snapshot();
        const path = await this.#client.path(this.#pathCommand(snapshot, this.#pathTarget));
        if (!same(snapshot, path)) {
          continue;
        }
        // Only for a consumer that asked for branches. Loading a page nobody
        // wanted would make every refresh pay for a projection it will not use.
        const branches =
          branchOffset === undefined
            ? undefined
            : await this.#client.branches(this.#branchCommand(snapshot, branchOffset));
        if (branches !== undefined && !same(snapshot, branches)) {
          continue;
        }
        if (this.#lifecycle.active && load === this.#load) {
          this.#install(snapshot, path, branches);
          this.#setStatus({ kind: "ready" });
        }
        return;
      }
      throw new ForkHistoryProjectionGapError();
    } catch (error) {
      if (this.#lifecycle.active && load === this.#load) {
        this.#setStatus({ kind: "failed", error });
      }
    }
  }

  async loadBranches(offset = 0): Promise<ForkBranchPageSnapshot> {
    const value = await this.#loadCurrent((snapshot) =>
      this.#client.branches(this.#branchCommand(snapshot, offset)),
    );
    this.#branches = value;
    this.#notify();
    return value;
  }

  /**
   * The continuations at one entry, or at the root when `anchorEntryId` is
   * null. Every child is returned, the one already on the current page
   * included: a renderer showing entry E inline drops it and shows the rest as
   * that entry's forks, which is why `continuationCount` is one more than the
   * fork badge.
   *
   * Not cached and not refreshed. A consumer asks when the operator opens an
   * entry, and the revision check below tells them if it went stale meanwhile.
   */
  loadContinuations(
    anchorEntryId: string | null,
    offset = 0,
  ): Promise<ForkContinuationPageSnapshot> {
    return this.#loadCurrent((snapshot) =>
      this.#client.continuations({
        protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
        authorityEpoch: snapshot.authorityEpoch,
        historyId: snapshot.summary.historyId,
        expectedRevision: snapshot.summary.revision,
        anchorEntryId,
        offset,
        limit: this.#continuationPageSize,
      }),
    );
  }

  /**
   * The flat run beginning at one entry. The same snapshot type as the main
   * path, so a renderer draws a nested fork with the component it already has.
   */
  loadContinuationRun(fromEntryId: string, offset = 0): Promise<ForkPathPageSnapshot> {
    return this.#loadCurrent((snapshot) =>
      this.#client.path({
        ...this.#pathCommand(snapshot, { kind: "continuation", fromEntryId }),
        offset,
      }),
    );
  }

  /**
   * Deletes one continuation and everything below it, then refreshes.
   *
   * **Irreversible.** Nothing restores what this removes. Confirm it with the
   * operator before calling; the authority will not ask.
   *
   * It refuses four things rather than working around them: the entry the
   * operator stands on or above, the active line, a pinned branch, and an
   * entry that does not exist. A refusal is returned as a typed result.
   */
  async deleteContinuation(entryId: string): Promise<ForkDeleteResult> {
    const snapshot = this.#required();
    const result = await this.#client.deleteContinuation({
      protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
      authorityEpoch: snapshot.authorityEpoch,
      historyId: snapshot.summary.historyId,
      expectedRevision: snapshot.summary.revision,
      entryId,
    });
    // Every page the consumer holds may name entries that no longer exist, so
    // this refresh is not optional and is not left to the changed event.
    if (result.status === "deleted" || result.rejection.refreshRequired) {
      await this.refresh();
    }
    return result;
  }

  /**
   * Prunes the unprotected share to a budget, then refreshes if anything went.
   *
   * The budget bounds what is **not** protected -- not on the current branch
   * and not on a pinned branch. Size it for the transient history to keep; a
   * graph with a large pinned set exceeds it and that is correct.
   *
   * Longhorn owns no scheduler. Call this on record, on a timer, on an
   * operator action, or not at all: those are the app's three triggers and
   * opting out is not calling it. `summary.retainedEntryCount` and
   * `summary.retainedEncodedWeight` are there to decide with.
   */
  async prune(maximumEntries: number, maximumEncodedWeight: number): Promise<ForkPruneResult> {
    const snapshot = this.#required();
    const result = await this.#client.prune({
      protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
      authorityEpoch: snapshot.authorityEpoch,
      historyId: snapshot.summary.historyId,
      expectedRevision: snapshot.summary.revision,
      maximumEntries,
      maximumEncodedWeight,
    });
    if (
      result.status === "pruned" ||
      (result.status === "rejected" && result.rejection.refreshRequired)
    ) {
      await this.refresh();
    }
    return result;
  }

  async selectDefaultPath(): Promise<void> {
    this.#pathTarget = { kind: "default" };
    await this.refresh();
  }
  async selectBranchPath(branchId: ForkBranchId): Promise<void> {
    this.#pathTarget = { kind: "branch", branchId };
    await this.refresh();
  }
  undo(): Promise<ForkNavigationResult> {
    return this.#navigate({ kind: "undo" });
  }
  redo(): Promise<ForkNavigationResult> {
    return this.#navigate({ kind: "redo" });
  }
  checkout(branchId: ForkBranchId, entryId: string): Promise<ForkNavigationResult> {
    return this.#navigate({ kind: "checkout", branchId, entryId });
  }
  /** Move to a branch's root, the position before its first entry. */
  checkoutBranchRoot(branchId: ForkBranchId): Promise<ForkNavigationResult> {
    return this.#navigate({ kind: "checkoutBranchRoot", branchId });
  }
  /**
   * Check out the run beginning at one entry: it becomes the current line,
   * and the previous line becomes a fork at the same entry. No delta of it is
   * applied, and the operator ends up standing at the fork entry whether or
   * not they started there.
   *
   * This commits. Browsing the forks does not — use `loadContinuationRun` to
   * show a fork's entries without choosing it.
   */
  checkoutContinuation(entryId: string): Promise<ForkNavigationResult> {
    return this.#navigate({ kind: "checkoutContinuation", entryId });
  }

  async #navigate(target: ForkNavigationTargetProjection): Promise<ForkNavigationResult> {
    const snapshot = this.#required();
    const navigation = ++this.#navigation;
    this.#pending = true;
    this.#notify();
    try {
      const result = await this.#client.navigate({
        protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
        authorityEpoch: snapshot.authorityEpoch,
        historyId: snapshot.summary.historyId,
        planId: this.#client.nextPlanId(),
        expectedRevision: snapshot.summary.revision,
        target,
      });
      if (!this.#lifecycle.active || navigation !== this.#navigation) {
        throw new ForkHistoryLateResultError();
      }
      const path = await this.#client.path(this.#pathCommand(result.snapshot, this.#pathTarget));
      if (!this.#lifecycle.active || navigation !== this.#navigation) {
        throw new ForkHistoryLateResultError();
      }
      if (!same(result.snapshot, path)) {
        throw new ForkHistoryProjectionGapError();
      }
      const branchOffset = this.#branches?.offset;
      const branches =
        branchOffset === undefined
          ? undefined
          : await this.#client.branches(this.#branchCommand(result.snapshot, branchOffset));
      if (!this.#lifecycle.active || navigation !== this.#navigation) {
        throw new ForkHistoryLateResultError();
      }
      if (branches !== undefined && !same(result.snapshot, branches)) {
        throw new ForkHistoryProjectionGapError();
      }
      this.#install(result.snapshot, path, branches);
      this.#setStatus({ kind: "ready" });
      return result;
    } catch (error) {
      // A superseded navigation no longer owns the controller's status.
      if (this.#lifecycle.active && navigation === this.#navigation) {
        this.#setStatus({ kind: "failed", error });
      }
      throw error;
    } finally {
      if (navigation === this.#navigation) {
        this.#pending = false;
        this.#notify();
      }
    }
  }

  /**
   * Loads one page against the held snapshot. The page is returned only while
   * the same lifetime still holds the revision it was read at.
   */
  async #loadCurrent<Page extends ForkPage>(
    load: (snapshot: ForkSnapshot) => Promise<Page>,
  ): Promise<Page> {
    const snapshot = this.#required();
    const lifecycle = this.#lifecycle.revision;
    const page = await load(snapshot);
    if (!this.#lifecycle.isCurrent(lifecycle)) {
      throw new ForkHistoryLateResultError();
    }
    const current = this.#snapshot;
    if (!same(snapshot, page) || current === undefined || !same(current, page)) {
      throw new ForkHistoryProjectionGapError();
    }
    return page;
  }

  #pathCommand(snapshot: ForkSnapshot, target: ForkPathTargetProjection) {
    return {
      protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
      authorityEpoch: snapshot.authorityEpoch,
      historyId: snapshot.summary.historyId,
      expectedRevision: snapshot.summary.revision,
      target,
      offset: 0,
      limit: this.#pathPageSize,
    } as const;
  }
  #required(): ForkSnapshot {
    if (!this.#lifecycle.active || this.#snapshot === undefined) {
      throw new ForkHistoryUnavailableError();
    }
    return this.#snapshot;
  }
  #branchCommand(snapshot: ForkSnapshot, offset: number) {
    return {
      protocolVersion: FORK_HISTORY_PROTOCOL_VERSION,
      authorityEpoch: snapshot.authorityEpoch,
      historyId: snapshot.summary.historyId,
      expectedRevision: snapshot.summary.revision,
      offset,
      limit: this.#branchPageSize,
    } as const;
  }
  #install(
    snapshot: ForkSnapshot,
    path: ForkPathPageSnapshot,
    branches?: ForkBranchPageSnapshot,
  ): void {
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
    this.#path = path;
    if (branches !== undefined) {
      this.#branches = branches;
    } else if (this.#branches !== undefined && !same(snapshot, this.#branches)) {
      this.#branches = undefined;
    }
  }
  #changed(event: ForkChangedEvent): void {
    const snapshot = this.#snapshot;
    if (
      !this.#lifecycle.active ||
      snapshot === undefined ||
      event.authorityEpoch !== snapshot.authorityEpoch ||
      event.historyId !== snapshot.summary.historyId ||
      event.committedRevision > snapshot.summary.revision
    ) {
      void this.refresh();
    }
  }
  #setStatus(status: ForkHistoryControllerStatus): void {
    this.#status = status;
    this.#notify();
  }
  #notify(): void {
    for (const observer of this.#observers) {
      observer();
    }
  }
}

type ForkPage = { authorityEpoch: number; historyId: string; revision: number };

function same(snapshot: ForkSnapshot, page: ForkPage): boolean {
  return (
    snapshot.authorityEpoch === page.authorityEpoch &&
    snapshot.summary.historyId === page.historyId &&
    snapshot.summary.revision === page.revision
  );
}
function pageSize(value: number): number {
  if (!Number.isSafeInteger(value) || value < 1 || value > MAXIMUM_FORK_HISTORY_PAGE_SIZE) {
    throw new RangeError(`fork-history page size must be 1..${MAXIMUM_FORK_HISTORY_PAGE_SIZE}`);
  }
  return value;
}
export class ForkHistoryUnavailableError extends Error {
  constructor(message = "fork-history controller is not ready") {
    super(message);
    this.name = "ForkHistoryUnavailableError";
  }
}

export class ForkHistoryProjectionGapError extends Error {
  constructor(message = "fork-history projections do not share one authority revision") {
    super(message);
    this.name = "ForkHistoryProjectionGapError";
  }
}

export class ForkHistoryLateResultError extends Error {
  constructor(message = "fork-history result arrived after the controller moved on") {
    super(message);
    this.name = "ForkHistoryLateResultError";
  }
}
