import { HistorySession } from "../src/svelte.ts";
import fixtureArtifact from "../../../fixtures/history/protocol-v1.json";
import type {
  HistoryNavigationCommand,
  HistoryPageCommand,
} from "../src/generated/protocol.ts";
import type { HistoryPort, HistoryUnlisten } from "../src/ports.ts";
import { clone, type HistoryFixture } from "../tests/support.ts";

export class MountedHistoryPort implements HistoryPort {
  readonly listeners = new Set<(event: unknown) => void>();
  readonly value = fixtureArtifact as HistoryFixture;
  snapshotValue = clone(this.value.snapshot);
  pageSnapshot = clone(this.value.page);
  result = clone(this.value.navigationResults[0]);
  unlistenCount = 0;
  #plan = 0;

  async snapshot(): Promise<unknown> {
    return clone(this.snapshotValue);
  }

  async page(_command: HistoryPageCommand): Promise<unknown> {
    return clone(this.pageSnapshot);
  }

  async navigate(_command: HistoryNavigationCommand): Promise<unknown> {
    this.snapshotValue = clone(this.result.snapshot);
    this.pageSnapshot = {
      ...this.pageSnapshot,
      revision: this.snapshotValue.summary.revision,
    };
    return clone(this.result);
  }

  listen(listener: (event: unknown) => void): HistoryUnlisten {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
      this.unlistenCount += 1;
    };
  }

  nextPlanId(): string {
    this.#plan += 1;
    return `plan:svelte-${this.#plan}`;
  }
}

export function createMountedSession() {
  const port = new MountedHistoryPort();
  const session = new HistorySession({ port, pageSize: 2 });
  return { port, session };
}
