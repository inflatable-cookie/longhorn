import { HistoryController } from "@inflatable-cookie/longhorn/history";
import type { HistoryControllerOptions } from "@inflatable-cookie/longhorn/history";
import type {
  HistoryEntryId,
  HistoryNavigationResult,
} from "@inflatable-cookie/longhorn/history/protocol";

import { freezeProjection } from "../../bindings/freeze-projection.ts";

export class HistorySession {
  readonly #controller: HistoryController;
  readonly #unobserve: () => void;
  #projection: ReturnType<typeof project>;

  constructor(options: HistoryControllerOptions) {
    this.#controller = new HistoryController(options);
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

  get pageSnapshot() {
    return this.#projection.pageSnapshot;
  }

  get entries() {
    return this.#projection.entries;
  }

  get rejection() {
    return this.#projection.rejection;
  }

  get filter() {
    return this.#projection.filter;
  }

  get page() {
    return this.#projection.page;
  }

  get pageSize() {
    return this.#projection.pageSize;
  }

  get totalEntries() {
    return this.#projection.totalEntries;
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

  start(): Promise<void> {
    return this.#controller.start();
  }

  stop(): Promise<void> {
    return this.#controller.stop();
  }

  refresh(): Promise<void> {
    return this.#controller.refresh();
  }

  async dispose(): Promise<void> {
    this.#unobserve();
    await this.#controller.stop();
  }

  setFilter(value: string): void {
    this.#controller.setFilter(value);
  }

  setPage(value: number): Promise<void> {
    return this.#controller.setPage(value);
  }

  setPageSize(value: number): Promise<void> {
    return this.#controller.setPageSize(value);
  }

  undo(): Promise<HistoryNavigationResult> {
    return this.#controller.undo();
  }

  redo(): Promise<HistoryNavigationResult> {
    return this.#controller.redo();
  }

  checkout(entryId: HistoryEntryId): Promise<HistoryNavigationResult> {
    return this.#controller.checkout(entryId);
  }

  /** Card 191. The position before the oldest retained entry, in one step. */
  checkoutRoot(): Promise<HistoryNavigationResult> {
    return this.#controller.checkoutRoot();
  }
}

function project(controller: HistoryController) {
  return {
    status: Object.freeze({ ...controller.status }),
    snapshot: freezeProjection(structuredClone(controller.snapshot)),
    pageSnapshot: freezeProjection(structuredClone(controller.pageSnapshot)),
    entries: freezeProjection(structuredClone(controller.entries)),
    rejection: freezeProjection(structuredClone(controller.rejection)),
    filter: controller.filter,
    page: controller.page,
    pageSize: controller.pageSize,
    totalEntries: controller.totalEntries,
    navigationPending: controller.navigationPending,
    canUndo: controller.canUndo,
    canRedo: controller.canRedo,
  };
}
