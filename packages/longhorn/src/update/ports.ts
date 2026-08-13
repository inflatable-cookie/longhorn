import type {
  UpdateChangedEvent,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateInstallCommand,
  UpdateOutcomeProjection,
  UpdateSelectChannelCommand,
  UpdateSnapshot,
} from "./generated/protocol.ts";

export type UpdateUnlisten = () => void | Promise<void>;

/**
 * The raw seam.
 *
 * Every result is `unknown`. What arrives over a transport is untrusted until
 * a validator says otherwise, and `UpdateClient` is what says so.
 */
export interface UpdatePort {
  snapshot(): Promise<unknown>;
  check(command: UpdateCheckCommand): Promise<unknown>;
  selectChannel(command: UpdateSelectChannelCommand): Promise<unknown>;
  defer(command: UpdateDeferCommand): Promise<unknown>;
  install(command: UpdateInstallCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): UpdateUnlisten | Promise<UpdateUnlisten>;
}

/** The same seam after validation. */
export interface CheckedUpdatePort {
  snapshot(): Promise<UpdateSnapshot>;
  check(command: UpdateCheckCommand): Promise<UpdateOutcomeProjection>;
  selectChannel(command: UpdateSelectChannelCommand): Promise<UpdateOutcomeProjection>;
  defer(command: UpdateDeferCommand): Promise<UpdateOutcomeProjection>;
  install(command: UpdateInstallCommand): Promise<UpdateOutcomeProjection>;
  listen(listener: (event: UpdateChangedEvent) => void): Promise<UpdateUnlisten>;
}
