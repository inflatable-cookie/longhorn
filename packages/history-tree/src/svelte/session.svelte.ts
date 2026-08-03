import { ForkHistoryController, type ForkHistoryControllerOptions, type ForkHistoryControllerStatus } from "../controller.ts";
import type { ForkBranchId, ForkBranchPageSnapshot, ForkEntryRecord, ForkNavigationResult, ForkPathPageSnapshot, ForkSnapshot } from "../generated/protocol.ts";
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
  #sync() { this.status = this.#controller.status; this.snapshot = this.#controller.snapshot; this.path = this.#controller.path; this.branches = this.#controller.branches; this.entries = [...this.#controller.entries]; this.navigationPending = this.#controller.navigationPending; this.canUndo = this.#controller.canUndo; this.canRedo = this.#controller.canRedo; }
}
