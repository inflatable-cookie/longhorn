import { isEventTransport } from "@inflatable-cookie/longhorn/core";
import type { EventTransport, InvokeTransport } from "@inflatable-cookie/longhorn/core";
import type {
  UpdateApplyCommand,
  UpdateCancelCommand,
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdatePort,
  UpdatePrepareCommand,
  UpdateSelectChannelCommand,
  UpdateUnlisten,
} from "@inflatable-cookie/longhorn/update";
import type { TauriTransport } from "./transport/index.ts";
export const UPDATE_SNAPSHOT_COMMAND = "longhorn_update_snapshot";
export const UPDATE_CHECK_COMMAND = "longhorn_update_check";
export const UPDATE_SELECT_CHANNEL_COMMAND = "longhorn_update_select_channel";
export const UPDATE_DEFER_COMMAND = "longhorn_update_defer";
export const UPDATE_PREPARE_COMMAND = "longhorn_update_prepare";
export const UPDATE_APPLY_COMMAND = "longhorn_update_apply";
export const UPDATE_CANCEL_COMMAND = "longhorn_update_cancel";
export const UPDATE_CHANGED_EVENT = "longhorn://update/changed";
/**
 * The update seam.
 *
 * Committed changes produce an invalidation hint. Live byte progress arrives
 * on a channel owned by the prepare invocation.
 *
 * Results are `unknown`, as every other raw port's are. What comes back over a
 * transport is untrusted until a validator says otherwise; `UpdateClient` is
 * what narrows them. Commands going *out* are typed, because those this side
 * constructs.
 */
export function createTauriUpdatePort(options: {
  readonly transport: InvokeTransport;
}): UpdatePort {
  const events = isEventTransport(options.transport) ? options.transport : undefined;
  const transport = options.transport;
  const progress = hasProgress(transport) ? transport : undefined;
  return {
    snapshot: () => transport.invoke(UPDATE_SNAPSHOT_COMMAND, {}),
    check: (command: UpdateCheckCommand) =>
      options.transport.invoke(UPDATE_CHECK_COMMAND, { command }),
    selectChannel: (command: UpdateSelectChannelCommand) =>
      options.transport.invoke(UPDATE_SELECT_CHANNEL_COMMAND, { command }),
    defer: (command: UpdateDeferCommand) =>
      options.transport.invoke(UPDATE_DEFER_COMMAND, { command }),
    prepare: (command: UpdatePrepareCommand) =>
      options.transport.invoke(UPDATE_PREPARE_COMMAND, { command }),
    prepareWithProgress:
      progress === undefined
        ? undefined
        : (command, listener) =>
            progress.invokeWithProgress(UPDATE_PREPARE_COMMAND, { command }, listener),
    apply: (command: UpdateApplyCommand) =>
      options.transport.invoke(UPDATE_APPLY_COMMAND, { command }),
    cancel: (command: UpdateCancelCommand) =>
      options.transport.invoke(UPDATE_CANCEL_COMMAND, { command }),
    listen:
      events === undefined
        ? undefined
        : (listener) => listen(events, UPDATE_CHANGED_EVENT, listener),
  };
}
function hasProgress(transport: InvokeTransport): transport is TauriTransport {
  return "invokeWithProgress" in transport && typeof transport.invokeWithProgress === "function";
}
function listen(
  events: EventTransport,
  event: string,
  listener: (event: unknown) => void,
): Promise<UpdateUnlisten> {
  return events.listen(event, listener);
}
