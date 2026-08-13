import type {
  UpdateChangedEvent,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateInstallCommand,
  UpdateOutcomeProjection,
  UpdateSelectChannelCommand,
  UpdateSnapshot,
} from "./generated/protocol.ts";
import type { CheckedUpdatePort, UpdatePort, UpdateUnlisten } from "./ports.ts";
import {
  assertUpdateChangedEvent,
  assertUpdateCheckCommand,
  assertUpdateDeferCommand,
  assertUpdateInstallCommand,
  assertUpdateOutcome,
  assertUpdateSelectChannelCommand,
  assertUpdateSnapshot,
} from "./validation.ts";

export class UpdateClient implements CheckedUpdatePort {
  constructor(readonly port: UpdatePort) {}
  async snapshot(): Promise<UpdateSnapshot> { const value = await this.port.snapshot(); assertUpdateSnapshot(value); return value; }
  async check(command: UpdateCheckCommand): Promise<UpdateOutcomeProjection> { assertUpdateCheckCommand(command); const value = await this.port.check(command); assertUpdateOutcome(value); return value; }
  async selectChannel(command: UpdateSelectChannelCommand): Promise<UpdateOutcomeProjection> { assertUpdateSelectChannelCommand(command); const value = await this.port.selectChannel(command); assertUpdateOutcome(value); return value; }
  async defer(command: UpdateDeferCommand): Promise<UpdateOutcomeProjection> { assertUpdateDeferCommand(command); const value = await this.port.defer(command); assertUpdateOutcome(value); return value; }
  async install(command: UpdateInstallCommand): Promise<UpdateOutcomeProjection> { assertUpdateInstallCommand(command); const value = await this.port.install(command); assertUpdateOutcome(value); return value; }
  async listen(listener: (event: UpdateChangedEvent) => void): Promise<UpdateUnlisten> { if (this.port.listen === undefined) return () => {}; return this.port.listen((value) => { assertUpdateChangedEvent(value); listener(value); }); }
}
