import { wireCopy } from "../core/wire-copy.ts";
import type { HistoryNavigationCommand, HistoryPageCommand } from "./generated/protocol.ts";
import type { HistoryPort, HistoryUnlisten } from "./ports.ts";

export class SerializedHistoryPort implements HistoryPort {
  readonly #inner: HistoryPort;

  constructor(inner: HistoryPort) {
    this.#inner = inner;
  }

  async snapshot(): Promise<unknown> {
    return wireCopy(await this.#inner.snapshot());
  }

  async page(command: HistoryPageCommand): Promise<unknown> {
    return wireCopy(await this.#inner.page(wireCopy(command)));
  }

  async navigate(command: HistoryNavigationCommand): Promise<unknown> {
    return wireCopy(await this.#inner.navigate(wireCopy(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<HistoryUnlisten> {
    if (this.#inner.listen === undefined) {
      return () => {};
    }
    return this.#inner.listen((event) => listener(wireCopy(event)));
  }

  nextPlanId(): string {
    return this.#inner.nextPlanId();
  }
}
