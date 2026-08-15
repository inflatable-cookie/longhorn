import { readFileSync } from "node:fs";

import type {
  LicenceActivateCommand,
  LicenceChangedEvent,
  LicenceDeactivateCommand,
  LicenceOutcomeProjection,
  LicenceRefreshCommand,
  LicenceReleaseSeatCommand,
  LicenceRenameSeatCommand,
  LicenceSnapshot,
} from "../../src/licence/generated/protocol.ts";

export interface LicenceFixture {
  readonly protocolVersion: number;
  readonly snapshot: LicenceSnapshot;
  readonly graceSnapshot: LicenceSnapshot;
  readonly unlicensedSnapshot: LicenceSnapshot;
  readonly activateCommands: readonly LicenceActivateCommand[];
  readonly deactivateCommand: LicenceDeactivateCommand;
  readonly refreshCommand: LicenceRefreshCommand;
  readonly releaseSeatCommand: LicenceReleaseSeatCommand;
  readonly renameSeatCommand: LicenceRenameSeatCommand;
  readonly outcomes: readonly LicenceOutcomeProjection[];
  readonly changedEvent: LicenceChangedEvent;
  readonly incompatibility: Record<string, unknown>;
}

export function fixture(): LicenceFixture {
  return JSON.parse(
    readFileSync(
      new URL("../../../../fixtures/licence/protocol-v1.json", import.meta.url),
      "utf8",
    ),
  ) as LicenceFixture;
}

export function clone<Value>(value: Value): Value {
  return JSON.parse(JSON.stringify(value)) as Value;
}
