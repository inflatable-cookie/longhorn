import { readFileSync } from "node:fs";

import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistorySnapshot,
} from "../../src/history/generated/protocol.ts";

export interface HistoryFixture {
  readonly protocolVersion: number;
  readonly snapshot: HistorySnapshot;
  readonly pageRequest: HistoryPageCommand;
  readonly page: HistoryPageSnapshot;
  readonly navigationCommand: HistoryNavigationCommand;
  readonly navigationResults: readonly HistoryNavigationResult[];
  readonly changedEvent: HistoryChangedEvent;
  readonly incompatibility: Record<string, unknown>;
}

export function fixture(): HistoryFixture {
  return JSON.parse(
    readFileSync(
      new URL("../../../../fixtures/history/protocol-v1.json", import.meta.url),
      "utf8",
    ),
  ) as HistoryFixture;
}

export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}

export function tick(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
