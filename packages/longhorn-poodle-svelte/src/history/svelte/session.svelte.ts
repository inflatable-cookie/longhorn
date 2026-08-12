import {
  HistoryController,
  type HistoryControllerOptions,
  type HistoryControllerStatus,
} from "@inflatable-cookie/longhorn/history";
import type {
  HistoryEntryId,
  HistoryEntryRecord,
  HistoryNavigationRejectionProjection,
  HistoryNavigationResult,
  HistoryPageSnapshot,
  HistorySnapshot,
} from "@inflatable-cookie/longhorn/history/protocol";

export class HistorySession {
  readonly #controller: HistoryController;
  readonly #unobserve: () => void;
  status = $state<HistoryControllerStatus>({ kind: "idle" });
  snapshot = $state<HistorySnapshot | undefined>();
  pageSnapshot = $state<HistoryPageSnapshot | undefined>();
  entries = $state<HistoryEntryRecord[]>([]);
  rejection = $state<HistoryNavigationRejectionProjection | undefined>();
  filter = $state("");
  page = $state(1);
  pageSize = $state(50);
  totalEntries = $state(0);
  navigationPending = $state(false);
  canUndo = $state(false);
  canRedo = $state(false);

  constructor(options: HistoryControllerOptions) {
    this.#controller = new HistoryController(options);
    this.#unobserve = this.#controller.observe(() => this.#sync());
    this.#sync();
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

  #sync(): void {
    this.status = this.#controller.status;
    this.snapshot = this.#controller.snapshot;
    this.pageSnapshot = this.#controller.pageSnapshot;
    this.entries = [...this.#controller.entries];
    this.rejection = this.#controller.rejection;
    this.filter = this.#controller.filter;
    this.page = this.#controller.page;
    this.pageSize = this.#controller.pageSize;
    this.totalEntries = this.#controller.totalEntries;
    this.navigationPending = this.#controller.navigationPending;
    this.canUndo = this.#controller.canUndo;
    this.canRedo = this.#controller.canRedo;
  }
}
