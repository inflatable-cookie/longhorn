import type {
  LicenceActivateCommand,
  LicenceChangedEvent,
  LicenceDeactivateCommand,
  LicenceOutcomeProjection,
  LicenceRefreshCommand,
  LicenceSnapshot,
} from "./generated/protocol.ts";
import type { CheckedLicencePort, LicencePort, LicenceUnlisten } from "./ports.ts";
import {
  assertLicenceActivateCommand,
  assertLicenceChangedEvent,
  assertLicenceDeactivateCommand,
  assertLicenceOutcome,
  assertLicenceRefreshCommand,
  assertLicenceSnapshot,
} from "./validation.ts";

export class LicenceClient implements CheckedLicencePort {
  constructor(readonly port: LicencePort) {}
  async snapshot(): Promise<LicenceSnapshot> { const value = await this.port.snapshot(); assertLicenceSnapshot(value); return value; }
  async activate(command: LicenceActivateCommand): Promise<LicenceOutcomeProjection> { assertLicenceActivateCommand(command); const value = await this.port.activate(command); assertLicenceOutcome(value); return value; }
  async deactivate(command: LicenceDeactivateCommand): Promise<LicenceOutcomeProjection> { assertLicenceDeactivateCommand(command); const value = await this.port.deactivate(command); assertLicenceOutcome(value); return value; }
  async refresh(command: LicenceRefreshCommand): Promise<LicenceOutcomeProjection> { assertLicenceRefreshCommand(command); const value = await this.port.refresh(command); assertLicenceOutcome(value); return value; }
  async listen(listener: (event: LicenceChangedEvent) => void): Promise<LicenceUnlisten> { if (this.port.listen === undefined) return () => {}; return this.port.listen((value) => { assertLicenceChangedEvent(value); listener(value); }); }
}
