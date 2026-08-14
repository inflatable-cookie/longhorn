import { isEventTransport, type EventTransport, type InvokeTransport } from "@inflatable-cookie/longhorn/core";
import type {
  LicenceActivateCommand,
  LicenceDeactivateCommand,
  LicencePort,
  LicenceRefreshCommand,
  LicenceReleaseSeatCommand,
  LicenceRenameSeatCommand,
  LicenceUnlisten,
} from "@inflatable-cookie/longhorn/licence";
export const LICENCE_SNAPSHOT_COMMAND = "longhorn_licence_snapshot";
export const LICENCE_ACTIVATE_COMMAND = "longhorn_licence_activate";
export const LICENCE_DEACTIVATE_COMMAND = "longhorn_licence_deactivate";
export const LICENCE_REFRESH_COMMAND = "longhorn_licence_refresh";
export const LICENCE_RELEASE_SEAT_COMMAND = "longhorn_licence_release_seat";
export const LICENCE_RENAME_SEAT_COMMAND = "longhorn_licence_rename_seat";
export const LICENCE_CHANGED_EVENT = "longhorn://licence/changed";
/**
 * The licence seam.
 *
 * Six commands over four host capabilities: reading state, re-checking the
 * lease, presenting a credential, and managing seats are different grants.
 * Results are `unknown`, as every raw port's are — `LicenceClient` is what
 * narrows them. Commands going out are typed, because this side builds those.
 */
export function createTauriLicencePort(options: { readonly transport: InvokeTransport }): LicencePort {
  const events = isEventTransport(options.transport) ? options.transport : undefined;
  return {
    snapshot: () => options.transport.invoke(LICENCE_SNAPSHOT_COMMAND, {}),
    activate: (command: LicenceActivateCommand) => options.transport.invoke(LICENCE_ACTIVATE_COMMAND, { command }),
    deactivate: (command: LicenceDeactivateCommand) => options.transport.invoke(LICENCE_DEACTIVATE_COMMAND, { command }),
    refresh: (command: LicenceRefreshCommand) => options.transport.invoke(LICENCE_REFRESH_COMMAND, { command }),
    releaseSeat: (command: LicenceReleaseSeatCommand) => options.transport.invoke(LICENCE_RELEASE_SEAT_COMMAND, { command }),
    renameSeat: (command: LicenceRenameSeatCommand) => options.transport.invoke(LICENCE_RENAME_SEAT_COMMAND, { command }),
    listen: events === undefined ? undefined : (listener) => listen(events, listener),
  };
}
function listen(events: EventTransport, listener: (event: unknown) => void): Promise<LicenceUnlisten> { return events.listen(LICENCE_CHANGED_EVENT, listener); }
