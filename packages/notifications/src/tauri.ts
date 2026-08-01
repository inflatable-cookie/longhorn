import { isEventTransport, type EventTransport, type InvokeTransport } from "@longhorn/core";

import type { NotificationPort } from "./ports.ts";

export const NOTIFICATION_SNAPSHOT_COMMAND = "longhorn_notifications_snapshot";
export const NOTIFICATION_MUTATE_COMMAND = "longhorn_notifications_mutate";
export const NOTIFICATION_CHANGED_EVENT = "longhorn://notifications/changed";

export interface TauriNotificationPortOptions {
  readonly transport: InvokeTransport;
  readonly nextRequestId: () => string;
}

export function createTauriNotificationPort(options: TauriNotificationPortOptions): NotificationPort {
  const events = isEventTransport(options.transport) ? options.transport : undefined;
  return {
    snapshot: (query) => options.transport.invoke(NOTIFICATION_SNAPSHOT_COMMAND, { query }),
    mutate: (command) => options.transport.invoke(NOTIFICATION_MUTATE_COMMAND, { command }),
    listen: events === undefined ? undefined : (listener) => listenChanged(events, listener),
    nextRequestId: options.nextRequestId,
  };
}

function listenChanged(transport: EventTransport, listener: (event: unknown) => void) {
  return transport.listen(NOTIFICATION_CHANGED_EVENT, listener);
}
