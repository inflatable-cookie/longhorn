import type {
  HistoryNavigationCommand,
  HistoryPageCommand,
} from "./generated/protocol.ts";
import type { HistoryPort, HistoryUnlisten } from "./ports.ts";

export class SerializedHistoryPort implements HistoryPort {
  readonly #inner: HistoryPort;

  constructor(inner: HistoryPort) {
    this.#inner = inner;
  }

  async snapshot(): Promise<unknown> {
    return clone(await this.#inner.snapshot());
  }

  async page(command: HistoryPageCommand): Promise<unknown> {
    return clone(await this.#inner.page(clone(command)));
  }

  async navigate(command: HistoryNavigationCommand): Promise<unknown> {
    return clone(await this.#inner.navigate(clone(command)));
  }

  async listen(listener: (event: unknown) => void): Promise<HistoryUnlisten> {
    if (this.#inner.listen === undefined) return () => {};
    return this.#inner.listen((event) => listener(clone(event)));
  }

  nextPlanId(): string {
    return this.#inner.nextPlanId();
  }
}

function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
