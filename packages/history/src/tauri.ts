import {
  isEventTransport,
  type EventTransport,
  type InvokeTransport,
} from "@longhorn/core";

import type { HistoryPort } from "./ports.ts";

export const HISTORY_SNAPSHOT_COMMAND = "longhorn_history_snapshot";
export const HISTORY_PAGE_COMMAND = "longhorn_history_page";
export const HISTORY_NAVIGATE_COMMAND = "longhorn_history_navigate";
export const HISTORY_CHANGED_EVENT = "longhorn://history/changed";

export interface TauriHistoryPortOptions {
  readonly transport: InvokeTransport;
  readonly nextPlanId: () => string;
}

export function createTauriHistoryPort(
  options: TauriHistoryPortOptions,
): HistoryPort {
  const events = isEventTransport(options.transport)
    ? options.transport
    : undefined;
  return {
    snapshot: () => options.transport.invoke(HISTORY_SNAPSHOT_COMMAND, {}),
    page: (command) =>
      options.transport.invoke(HISTORY_PAGE_COMMAND, { command }),
    navigate: (command) =>
      options.transport.invoke(HISTORY_NAVIGATE_COMMAND, { command }),
    listen:
      events === undefined
        ? undefined
        : (listener) => listenChanged(events, listener),
    nextPlanId: options.nextPlanId,
  };
}

async function listenChanged(
  transport: EventTransport,
  listener: (event: unknown) => void,
) {
  return transport.listen(HISTORY_CHANGED_EVENT, listener);
}
