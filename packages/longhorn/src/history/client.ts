import {
  assertCompatibleHistoryChangedEvent,
  assertCompatibleHistoryNavigationCommand,
  assertCompatibleHistoryNavigationResult,
  assertCompatibleHistoryPageCommand,
  assertCompatibleHistoryPageSnapshot,
  assertCompatibleHistorySnapshot,
} from "./validation.ts";
import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistorySnapshot,
} from "./generated/protocol.ts";
import type {
  CheckedHistoryPort,
  HistoryPort,
  HistoryUnlisten,
} from "./ports.ts";

export class HistoryClient implements CheckedHistoryPort {
  readonly #port: HistoryPort;

  constructor(port: HistoryPort) {
    this.#port = port;
  }

  nextPlanId(): string {
    return this.#port.nextPlanId();
  }

  async snapshot(): Promise<HistorySnapshot> {
    const value = await this.#port.snapshot();
    assertCompatibleHistorySnapshot(value);
    return value;
  }

  async page(command: HistoryPageCommand): Promise<HistoryPageSnapshot> {
    assertCompatibleHistoryPageCommand(command);
    const value = await this.#port.page(command);
    assertCompatibleHistoryPageSnapshot(value);
    return value;
  }

  async navigate(
    command: HistoryNavigationCommand,
  ): Promise<HistoryNavigationResult> {
    assertCompatibleHistoryNavigationCommand(command);
    const value = await this.#port.navigate(command);
    assertCompatibleHistoryNavigationResult(value);
    return value;
  }

  async listen(
    listener: (event: HistoryChangedEvent) => void,
  ): Promise<HistoryUnlisten> {
    if (this.#port.listen === undefined) return () => {};
    return this.#port.listen((value) => {
      assertCompatibleHistoryChangedEvent(value);
      listener(value);
    });
  }
}
