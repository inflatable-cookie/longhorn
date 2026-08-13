import { isEventTransport, type EventTransport, type InvokeTransport } from "@inflatable-cookie/longhorn/core";
import type {
  UpdateCheckCommand,
  UpdateDeferCommand,
  UpdateInstallCommand,
  UpdatePort,
  UpdateSelectChannelCommand,
  UpdateUnlisten,
} from "@inflatable-cookie/longhorn/update";
export const UPDATE_SNAPSHOT_COMMAND = "longhorn_update_snapshot";
export const UPDATE_CHECK_COMMAND = "longhorn_update_check";
export const UPDATE_SELECT_CHANNEL_COMMAND = "longhorn_update_select_channel";
export const UPDATE_DEFER_COMMAND = "longhorn_update_defer";
export const UPDATE_INSTALL_COMMAND = "longhorn_update_install";
export const UPDATE_CHANGED_EVENT = "longhorn://update/changed";
/**
 * The update seam.
 *
 * Four commands and a read, mirroring `UpdateController`. Each is its own
 * invoke because each is its own capability on the host side: reading state,
 * reaching the network, changing what this install follows, and replacing the
 * running application are four different grants.
 *
 * Results are `unknown`, as every other raw port's are. What comes back over a
 * transport is untrusted until a validator says otherwise; `UpdateClient` is
 * what narrows them. Commands going *out* are typed, because those this side
 * constructs.
 */
export function createTauriUpdatePort(options: { readonly transport: InvokeTransport }): UpdatePort {
  const events = isEventTransport(options.transport) ? options.transport : undefined;
  return {
    snapshot: () => options.transport.invoke(UPDATE_SNAPSHOT_COMMAND, {}),
    check: (command) => options.transport.invoke(UPDATE_CHECK_COMMAND, { command }),
    selectChannel: (command) => options.transport.invoke(UPDATE_SELECT_CHANNEL_COMMAND, { command }),
    defer: (command) => options.transport.invoke(UPDATE_DEFER_COMMAND, { command }),
    install: (command) => options.transport.invoke(UPDATE_INSTALL_COMMAND, { command }),
    listen: events === undefined ? undefined : (listener) => listen(events, listener),
  };
}
function listen(events: EventTransport, listener: (event: unknown) => void): Promise<UpdateUnlisten> { return events.listen(UPDATE_CHANGED_EVENT, listener); }
