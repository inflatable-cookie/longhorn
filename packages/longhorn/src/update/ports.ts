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
  prepare(command: UpdatePrepareCommand): Promise<unknown>;
  apply(command: UpdateApplyCommand): Promise<unknown>;
  cancel(command: UpdateCancelCommand): Promise<unknown>;
  listen?(listener: (event: unknown) => void): UpdateUnlisten | Promise<UpdateUnlisten>;
  listenProgress?(listener: (event: unknown) => void): UpdateUnlisten | Promise<UpdateUnlisten>;
}

/** The same seam after validation. */
export interface CheckedUpdatePort {
  snapshot(): Promise<UpdateSnapshot>;
  check(command: UpdateCheckCommand): Promise<UpdateOutcomeProjection>;
  selectChannel(command: UpdateSelectChannelCommand): Promise<UpdateOutcomeProjection>;
  defer(command: UpdateDeferCommand): Promise<UpdateOutcomeProjection>;
  prepare(command: UpdatePrepareCommand): Promise<UpdateOutcomeProjection>;
  apply(command: UpdateApplyCommand): Promise<UpdateOutcomeProjection>;
  cancel(command: UpdateCancelCommand): Promise<UpdateOutcomeProjection>;
  listen(listener: (event: UpdateChangedEvent) => void): Promise<UpdateUnlisten>;
  listenProgress(listener: (event: UpdateProgressEvent) => void): Promise<UpdateUnlisten>;
}
