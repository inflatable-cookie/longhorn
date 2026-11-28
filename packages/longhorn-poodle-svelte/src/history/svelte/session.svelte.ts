import { HistoryController } from "@inflatable-cookie/longhorn/history";
import type { HistoryControllerOptions } from "@inflatable-cookie/longhorn/history";
import type {
  HistoryEntryId,
  HistoryNavigationResult,
} from "@inflatable-cookie/longhorn/history/protocol";

import { freezeProjection } from "../../bindings/freeze-projection.ts";

import { SessionProjection } from "../../bindings/session-projection.svelte.ts";

export class HistorySession {
  readonly #controller: HistoryController;
  readonly #projection: SessionProjection<ReturnType<typeof project>>;

  constructor(options: HistoryControllerOptions) {
    this.#controller = new HistoryController(options);
    this.#projection = new SessionProjection(this.#controller, () => project(this.#controller));
  }

  get status() {
    return this.#projection.current.status;
  }

  get snapshot() {
    return this.#projection.current.snapshot;
  }

  get pageSnapshot() {
    return this.#projection.current.pageSnapshot;
  }

  get entries() {
    return this.#projection.current.entries;
  }

  get rejection() {
    return this.#projection.current.rejection;
  }

  get filter() {
    return this.#projection.current.filter;
  }

  get page() {
    return this.#projection.current.page;
  }

  get pageSize() {
    return this.#projection.current.pageSize;
  }

  get totalEntries() {
    return this.#projection.current.totalEntries;
  }

  get navigationPending() {
    return this.#projection.current.navigationPending;
  }

  get canUndo() {
    return this.#projection.current.canUndo;
  }

  get canRedo() {
    return this.#projection.current.canRedo;
  }

  start(): Promise<void> {
    return this.#projection.start();
  }

  stop(): Promise<void> {
    return this.#projection.stop();
  }

  refresh(): Promise<void> {
    return this.#controller.refresh();
  }

  async dispose(): Promise<void> {
    await this.#projection.dispose();
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
