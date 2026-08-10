import {
  assertValidHistoryChangedEvent,
  assertValidHistoryNavigationCommand,
  assertValidHistoryNavigationResult,
  assertValidHistoryPageCommand,
  assertValidHistoryPageSnapshot,
  assertValidHistorySnapshot,
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
    assertValidHistorySnapshot(value);
    return value;
  }

  async page(command: HistoryPageCommand): Promise<HistoryPageSnapshot> {
    assertValidHistoryPageCommand(command);
    const value = await this.#port.page(command);
    assertValidHistoryPageSnapshot(value);
    return value;
  }

  async navigate(
    command: HistoryNavigationCommand,
  ): Promise<HistoryNavigationResult> {
    assertValidHistoryNavigationCommand(command);
    const value = await this.#port.navigate(command);
    assertValidHistoryNavigationResult(value);
    return value;
  }

  async listen(
    listener: (event: HistoryChangedEvent) => void,
  ): Promise<HistoryUnlisten> {
    if (this.#port.listen === undefined) return () => {};
    return this.#port.listen((value) => {
      assertValidHistoryChangedEvent(value);
      listener(value);
    });
  }
}
