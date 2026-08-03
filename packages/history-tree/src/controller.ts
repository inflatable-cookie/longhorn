import { ForkHistoryClient } from "./client.ts";
import { FORK_HISTORY_PROTOCOL_VERSION, type ForkBranchId, type ForkBranchPageSnapshot, type ForkChangedEvent, type ForkEntryRecord, type ForkNavigationResult, type ForkNavigationTargetProjection, type ForkPathPageSnapshot, type ForkPathTargetProjection, type ForkSnapshot } from "./generated/protocol.ts";
import type { ForkHistoryPort, ForkHistoryUnlisten } from "./ports.ts";

export type ForkHistoryControllerStatus = { readonly kind: "idle" } | { readonly kind: "loading" } | { readonly kind: "ready" } | { readonly kind: "failed"; readonly error: unknown };
export interface ForkHistoryControllerOptions { readonly port: ForkHistoryPort; readonly pathPageSize?: number; readonly branchPageSize?: number; }

export class ForkHistoryController {
  readonly #client: ForkHistoryClient;
  readonly #observers = new Set<() => void>();
  readonly #pathPageSize: number;
  readonly #branchPageSize: number;
  #status: ForkHistoryControllerStatus = { kind: "idle" };
  #snapshot?: ForkSnapshot;
  #path?: ForkPathPageSnapshot;
  #branches?: ForkBranchPageSnapshot;
  #pathTarget: ForkPathTargetProjection = { kind: "default" };
  #started = false;
  #lifecycle = 0;
  #load = 0;
  #navigation = 0;
  #pending = false;
  #unlisten: ForkHistoryUnlisten[] = [];

  constructor(options: ForkHistoryControllerOptions) { this.#client = new ForkHistoryClient(options.port); this.#pathPageSize = pageSize(options.pathPageSize ?? 50); this.#branchPageSize = pageSize(options.branchPageSize ?? 50); }
  get status() { return this.#status; }
  get snapshot() { return this.#snapshot; }
  get path() { return this.#path; }
  get branches() { return this.#branches; }
  get entries(): readonly ForkEntryRecord[] { return this.#path?.entries ?? []; }
  get navigationPending() { return this.#pending; }
  get canUndo() { return (this.#snapshot?.summary.undoDepth ?? 0) > 0; }
  get canRedo() { return (this.#snapshot?.summary.redoDepth ?? 0) > 0; }
  observe(observer: () => void): () => void { this.#observers.add(observer); return () => this.#observers.delete(observer); }

  async start(): Promise<void> {
    if (this.#started) return;
    this.#started = true; const lifecycle = ++this.#lifecycle; this.#setStatus({ kind: "loading" });
    try {
      const unlisten = await this.#client.listen((event) => this.#changed(event));
      if (!this.#started || lifecycle !== this.#lifecycle) { await unlisten(); return; }
      this.#unlisten.push(unlisten); await this.refresh();
    } catch (error) { if (this.#started && lifecycle === this.#lifecycle) this.#setStatus({ kind: "failed", error }); }
  }

  async stop(): Promise<void> {
    this.#started = false; this.#lifecycle += 1; this.#load += 1; this.#navigation += 1;
    await Promise.allSettled(this.#unlisten.splice(0).map((dispose) => dispose()));
    this.#snapshot = undefined; this.#path = undefined; this.#branches = undefined; this.#pathTarget = { kind: "default" }; this.#pending = false; this.#setStatus({ kind: "idle" });
  }

  async refresh(): Promise<void> {
    if (!this.#started) return;
    const load = ++this.#load;
    try {
      for (let attempt = 0; attempt < 2; attempt += 1) {
        const snapshot = await this.#client.snapshot();
        const path = await this.#client.path(this.#pathCommand(snapshot, this.#pathTarget));
        if (same(snapshot, path)) { if (this.#started && load === this.#load) { this.#install(snapshot, path); this.#setStatus({ kind: "ready" }); } return; }
      }
      throw new ForkHistoryProjectionGapError();
    } catch (error) { if (this.#started && load === this.#load) this.#setStatus({ kind: "failed", error }); }
  }

  async loadBranches(offset = 0): Promise<ForkBranchPageSnapshot> {
    const snapshot = this.#required();
    const value = await this.#client.branches({ protocolVersion: FORK_HISTORY_PROTOCOL_VERSION, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, offset, limit: this.#branchPageSize });
    if (!same(snapshot, value)) throw new ForkHistoryProjectionGapError();
    this.#branches = value; this.#notify(); return value;
  }

  async selectDefaultPath(): Promise<void> { this.#pathTarget = { kind: "default" }; await this.refresh(); }
  async selectBranchPath(branchId: ForkBranchId): Promise<void> { this.#pathTarget = { kind: "branch", branchId }; await this.refresh(); }
  undo(): Promise<ForkNavigationResult> { return this.#navigate({ kind: "undo" }); }
  redo(): Promise<ForkNavigationResult> { return this.#navigate({ kind: "redo" }); }
  checkout(branchId: ForkBranchId, entryId: string): Promise<ForkNavigationResult> { return this.#navigate({ kind: "checkout", branchId, entryId }); }

  async #navigate(target: ForkNavigationTargetProjection): Promise<ForkNavigationResult> {
    const snapshot = this.#required(); const navigation = ++this.#navigation; this.#pending = true; this.#notify();
    try {
      const result = await this.#client.navigate({ protocolVersion: FORK_HISTORY_PROTOCOL_VERSION, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, planId: this.#client.nextPlanId(), expectedRevision: snapshot.summary.revision, target });
      if (!this.#started || navigation !== this.#navigation) throw new ForkHistoryLateResultError();
      const path = await this.#client.path(this.#pathCommand(result.snapshot, this.#pathTarget));
      if (!this.#started || navigation !== this.#navigation) throw new ForkHistoryLateResultError();
      if (!same(result.snapshot, path)) throw new ForkHistoryProjectionGapError();
      this.#install(result.snapshot, path); this.#setStatus({ kind: "ready" }); return result;
    } finally { if (navigation === this.#navigation) { this.#pending = false; this.#notify(); } }
  }

  #pathCommand(snapshot: ForkSnapshot, target: ForkPathTargetProjection) { return { protocolVersion: FORK_HISTORY_PROTOCOL_VERSION, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, target, offset: 0, limit: this.#pathPageSize } as const; }
  #required(): ForkSnapshot { if (!this.#started || this.#snapshot === undefined) throw new ForkHistoryUnavailableError(); return this.#snapshot; }
  #install(snapshot: ForkSnapshot, path: ForkPathPageSnapshot): void { const current = this.#snapshot; if (current !== undefined && snapshot.authorityEpoch === current.authorityEpoch && snapshot.summary.historyId === current.summary.historyId && snapshot.summary.revision < current.summary.revision) return; this.#snapshot = snapshot; this.#path = path; if (this.#branches !== undefined && !same(snapshot, this.#branches)) this.#branches = undefined; this.#notify(); }
  #changed(event: ForkChangedEvent): void { const snapshot = this.#snapshot; if (!this.#started || snapshot === undefined || event.authorityEpoch !== snapshot.authorityEpoch || event.historyId !== snapshot.summary.historyId || event.committedRevision > snapshot.summary.revision) void this.refresh(); }
  #setStatus(status: ForkHistoryControllerStatus): void { this.#status = status; this.#notify(); }
  #notify(): void { for (const observer of this.#observers) observer(); }
}

function same(snapshot: ForkSnapshot, page: { authorityEpoch: number; historyId: string; revision: number }): boolean { return snapshot.authorityEpoch === page.authorityEpoch && snapshot.summary.historyId === page.historyId && snapshot.summary.revision === page.revision; }
function pageSize(value: number): number { if (!Number.isSafeInteger(value) || value < 1 || value > 256) throw new RangeError("fork-history page size must be 1..256"); return value; }
export class ForkHistoryUnavailableError extends Error {}
export class ForkHistoryProjectionGapError extends Error {}
export class ForkHistoryLateResultError extends Error {}
