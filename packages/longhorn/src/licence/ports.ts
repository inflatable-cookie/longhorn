import type {
  LicenceActivateCommand,
  LicenceChangedEvent,
  LicenceDeactivateCommand,
  LicenceOutcomeProjection,
  LicenceRefreshCommand,
  LicenceReleaseSeatCommand,
  LicenceRenameSeatCommand,
  LicenceSnapshot,
} from "./generated/protocol.ts";

export type LicenceUnlisten = () => void | Promise<void>;

/**
 * The raw seam.
 *
 * Every result is `unknown`. What arrives over a transport is untrusted until
 * a validator says otherwise, and `LicenceClient` is what says so.
 */
export interface LicencePort {
  snapshot(): Promise<unknown>;
  activate(command: LicenceActivateCommand): Promise<unknown>;
  deactivate(command: LicenceDeactivateCommand): Promise<unknown>;
  refresh(command: LicenceRefreshCommand): Promise<unknown>;
  releaseSeat(command: LicenceReleaseSeatCommand): Promise<unknown>;
  renameSeat(command: LicenceRenameSeatCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): LicenceUnlisten | Promise<LicenceUnlisten>;
}

/** The same seam after validation. */
export interface CheckedLicencePort {
  snapshot(): Promise<LicenceSnapshot>;
  activate(command: LicenceActivateCommand): Promise<LicenceOutcomeProjection>;
  deactivate(command: LicenceDeactivateCommand): Promise<LicenceOutcomeProjection>;
  refresh(command: LicenceRefreshCommand): Promise<LicenceOutcomeProjection>;
  releaseSeat(command: LicenceReleaseSeatCommand): Promise<LicenceOutcomeProjection>;
  renameSeat(command: LicenceRenameSeatCommand): Promise<LicenceOutcomeProjection>;
  listen(listener: (event: LicenceChangedEvent) => void): Promise<LicenceUnlisten>;
}
