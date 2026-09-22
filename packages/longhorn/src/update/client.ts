import type {
  UpdateApplyCommand,
  UpdateCancelCommand,
  UpdateChangedEvent,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateOutcomeProjection,
  UpdatePrepareCommand,
  UpdateProgressEvent,
  UpdateSelectChannelCommand,
  UpdateSnapshot,
} from "./generated/protocol.ts";
import type { CheckedUpdatePort, UpdatePort, UpdateUnlisten } from "./ports.ts";
import {
  assertUpdateApplyCommand,
  assertUpdateCancelCommand,
  assertUpdateChangedEvent,
  assertUpdateCheckCommand,
  assertUpdateDeferCommand,
  assertUpdateOutcome,
  assertUpdatePrepareCommand,
  assertUpdateProgressEvent,
  assertUpdateSelectChannelCommand,
  assertUpdateSnapshot,
} from "./validation.ts";

export class UpdateClient implements CheckedUpdatePort {
  constructor(readonly port: UpdatePort) {}
  async snapshot(): Promise<UpdateSnapshot> { const value = await this.port.snapshot(); assertUpdateSnapshot(value); return value; }
  async check(command: UpdateCheckCommand): Promise<UpdateOutcomeProjection> { assertUpdateCheckCommand(command); const value = await this.port.check(command); assertUpdateOutcome(value); return value; }
  async selectChannel(command: UpdateSelectChannelCommand): Promise<UpdateOutcomeProjection> { assertUpdateSelectChannelCommand(command); const value = await this.port.selectChannel(command); assertUpdateOutcome(value); return value; }
  async defer(command: UpdateDeferCommand): Promise<UpdateOutcomeProjection> { assertUpdateDeferCommand(command); const value = await this.port.defer(command); assertUpdateOutcome(value); return value; }
  async prepare(command: UpdatePrepareCommand): Promise<UpdateOutcomeProjection> { assertUpdatePrepareCommand(command); const value = await this.port.prepare(command); assertUpdateOutcome(value); return value; }
  async apply(command: UpdateApplyCommand): Promise<UpdateOutcomeProjection> { assertUpdateApplyCommand(command); const value = await this.port.apply(command); assertUpdateOutcome(value); return value; }
  async cancel(command: UpdateCancelCommand): Promise<UpdateOutcomeProjection> { assertUpdateCancelCommand(command); const value = await this.port.cancel(command); assertUpdateOutcome(value); return value; }
  async listen(listener: (event: UpdateChangedEvent) => void): Promise<UpdateUnlisten> { if (this.port.listen === undefined) return () => {}; return this.port.listen((value) => { assertUpdateChangedEvent(value); listener(value); }); }
  async listenProgress(listener: (event: UpdateProgressEvent) => void): Promise<UpdateUnlisten> { if (this.port.listenProgress === undefined) return () => {}; return this.port.listenProgress((value) => { assertUpdateProgressEvent(value); listener(value); }); }
}
