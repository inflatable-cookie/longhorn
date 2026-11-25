import { ForkHistoryController } from "@inflatable-cookie/longhorn/history-tree";
import type { ForkHistoryControllerOptions } from "@inflatable-cookie/longhorn/history-tree";
import type {
  ForkBranchId,
  ForkContinuationPageSnapshot,
  ForkPruneResult,
  ForkDeleteResult,
  ForkNavigationResult,
  ForkPathPageSnapshot,
} from "@inflatable-cookie/longhorn/history-tree/protocol";
import { freezeProjection } from "../../bindings/freeze-projection.ts";

export class ForkHistorySession {
  readonly #controller: ForkHistoryController;
  readonly #unobserve: () => void;
  #projection: ReturnType<typeof project>;

  constructor(options: ForkHistoryControllerOptions) {
    this.#controller = new ForkHistoryController(options);
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

  get path() {
    return this.#projection.path;
  }

  get branches() {
    return this.#projection.branches;
  }

  get entries() {
    return this.#projection.entries;
  }

  get navigationPending() {
    return this.#projection.navigationPending;
  }

  get canUndo() {
    return this.#projection.canUndo;
  }

  get canRedo() {
    return this.#projection.canRedo;
  }

  start() {
    return this.#controller.start();
  }
  stop() {
    return this.#controller.stop();
  }
  refresh() {
    return this.#controller.refresh();
  }
  async dispose() {
    this.#unobserve();
    await this.#controller.stop();
  }
  loadBranches(offset = 0) {
    return this.#controller.loadBranches(offset);
  }
  selectDefaultPath() {
    return this.#controller.selectDefaultPath();
  }
  selectBranchPath(branchId: ForkBranchId) {
    return this.#controller.selectBranchPath(branchId);
  }
  undo(): Promise<ForkNavigationResult> {
    return this.#controller.undo();
  }
  redo(): Promise<ForkNavigationResult> {
    return this.#controller.redo();
  }
  checkout(branchId: ForkBranchId, entryId: string) {
    return this.#controller.checkout(branchId, entryId);
  }
  /** Card 181. The position before a branch's first entry; a nascent branch sits here. */
  checkoutBranchRoot(branchId: ForkBranchId) {
    return this.#controller.checkoutBranchRoot(branchId);
  }
  /** Card 183. Every entry continuing from `anchorEntryId`, or from the root when null. */
  loadContinuations(
    anchorEntryId: string | null,
    offset = 0,
  ): Promise<ForkContinuationPageSnapshot> {
    return this.#controller.loadContinuations(anchorEntryId, offset);
  }
  /** Card 183. The flat run beginning at one entry, as the same page type as `path`. */
  loadContinuationRun(fromEntryId: string, offset = 0): Promise<ForkPathPageSnapshot> {
    return this.#controller.loadContinuationRun(fromEntryId, offset);
  }
  /** Card 186. Prune the unprotected share to a budget. */
  prune(maximumEntries: number, maximumEncodedWeight: number): Promise<ForkPruneResult> {
    return this.#controller.prune(maximumEntries, maximumEncodedWeight);
  }
  /** Card 185. Delete one continuation and everything below it. Irreversible. */
  deleteContinuation(entryId: string): Promise<ForkDeleteResult> {
    return this.#controller.deleteContinuation(entryId);
  }
  /** Card 184. Check out the run beginning at one entry; applies none of it. */
  checkoutContinuation(entryId: string): Promise<ForkNavigationResult> {
    return this.#controller.checkoutContinuation(entryId);
  }
}

function project(controller: ForkHistoryController) {
  return {
    status: Object.freeze({ ...controller.status }),
    snapshot: freezeProjection(structuredClone(controller.snapshot)),
    path: freezeProjection(structuredClone(controller.path)),
    branches: freezeProjection(structuredClone(controller.branches)),
    entries: freezeProjection(structuredClone(controller.entries)),
    navigationPending: controller.navigationPending,
    canUndo: controller.canUndo,
    canRedo: controller.canRedo,
  };
}
