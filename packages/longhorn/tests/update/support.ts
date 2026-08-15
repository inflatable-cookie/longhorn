import { readFileSync } from "node:fs";

import type {
  UpdateChangedEvent,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateInstallCommand,
  UpdateOutcomeProjection,
  UpdateSelectChannelCommand,
  UpdateSnapshot,
} from "../../src/update/generated/protocol.ts";

export interface UpdateFixture {
  readonly protocolVersion: number;
  readonly snapshot: UpdateSnapshot;
  readonly managedSnapshot: UpdateSnapshot;
  readonly aheadSnapshot: UpdateSnapshot;
  readonly withheldSnapshot: UpdateSnapshot;
  readonly upToDateSnapshot: UpdateSnapshot;
  readonly checkCommand: UpdateCheckCommand;
  readonly selectChannelCommand: UpdateSelectChannelCommand;
  readonly deferCommand: UpdateDeferCommand;
  readonly installCommand: UpdateInstallCommand;
  readonly outcomes: readonly UpdateOutcomeProjection[];
  readonly changedEvent: UpdateChangedEvent;
  readonly incompatibility: Record<string, unknown>;
}

export function fixture(): UpdateFixture {
  return JSON.parse(
    readFileSync(
      new URL("../../../../fixtures/update/protocol-v1.json", import.meta.url),
      "utf8",
    ),
  ) as UpdateFixture;
}

export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
