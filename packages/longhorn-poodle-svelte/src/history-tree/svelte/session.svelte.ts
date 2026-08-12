import { ForkHistoryController, type ForkHistoryControllerOptions, type ForkHistoryControllerStatus } from "@inflatable-cookie/longhorn/history-tree";
import type { ForkBranchId, ForkBranchPageSnapshot, ForkContinuationPageSnapshot, ForkEntryRecord, ForkRemovalReceiptProjection, ForkNavigationResult, ForkPathPageSnapshot, ForkSnapshot } from "@inflatable-cookie/longhorn/history-tree/protocol";
export class ForkHistorySession {
  readonly #controller: ForkHistoryController; readonly #unobserve: () => void;
  status = $state<ForkHistoryControllerStatus>({ kind: "idle" }); snapshot = $state<ForkSnapshot>(); path = $state<ForkPathPageSnapshot>(); branches = $state<ForkBranchPageSnapshot>(); entries = $state<ForkEntryRecord[]>([]); navigationPending = $state(false); canUndo = $state(false); canRedo = $state(false);
  constructor(options: ForkHistoryControllerOptions) { this.#controller = new ForkHistoryController(options); this.#unobserve = this.#controller.observe(() => this.#sync()); this.#sync(); }
  start() { return this.#controller.start(); } stop() { return this.#controller.stop(); } refresh() { return this.#controller.refresh(); }
  async dispose() { this.#unobserve(); await this.#controller.stop(); }
  loadBranches(offset = 0) { return this.#controller.loadBranches(offset); }
  selectDefaultPath() { return this.#controller.selectDefaultPath(); }
  selectBranchPath(branchId: ForkBranchId) { return this.#controller.selectBranchPath(branchId); }
  undo(): Promise<ForkNavigationResult> { return this.#controller.undo(); } redo(): Promise<ForkNavigationResult> { return this.#controller.redo(); }
  checkout(branchId: ForkBranchId, entryId: string) { return this.#controller.checkout(branchId, entryId); }
  /** Card 181. The position before a branch's first entry; a nascent branch sits here. */
  checkoutBranchRoot(branchId: ForkBranchId) { return this.#controller.checkoutBranchRoot(branchId); }
  /** Card 183. Every entry continuing from `anchorEntryId`, or from the root when null. */
  loadContinuations(anchorEntryId: string | null, offset = 0): Promise<ForkContinuationPageSnapshot> { return this.#controller.loadContinuations(anchorEntryId, offset); }
  /** Card 183. The flat run beginning at one entry, as the same page type as `path`. */
  loadContinuationRun(fromEntryId: string, offset = 0): Promise<ForkPathPageSnapshot> { return this.#controller.loadContinuationRun(fromEntryId, offset); }
  /** Card 185. Delete one continuation and everything below it. Irreversible. */
  deleteContinuation(entryId: string): Promise<ForkRemovalReceiptProjection> { return this.#controller.deleteContinuation(entryId); }
  /** Card 184. Check out the run beginning at one entry; applies none of it. */
  checkoutContinuation(entryId: string): Promise<ForkNavigationResult> { return this.#controller.checkoutContinuation(entryId); }
  #sync() { this.status = this.#controller.status; this.snapshot = this.#controller.snapshot; this.path = this.#controller.path; this.branches = this.#controller.branches; this.entries = [...this.#controller.entries]; this.navigationPending = this.#controller.navigationPending; this.canUndo = this.#controller.canUndo; this.canRedo = this.#controller.canRedo; }
}
