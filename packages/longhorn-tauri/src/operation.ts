import {
  isEventTransport,
  type EventTransport,
  type InvokeTransport,
} from "@inflatable-cookie/longhorn/core";

import type { OperationPort } from "@inflatable-cookie/longhorn/operation";

export const OPERATION_SNAPSHOT_COMMAND = "longhorn_operation_snapshot";
export const OPERATION_MUTATE_COMMAND = "longhorn_operation_mutate";
export const OPERATION_CANCEL_COMMAND = "longhorn_operation_cancel";
export const OPERATION_CHANGED_EVENT = "longhorn://operation/changed";

export interface TauriOperationPortOptions {
  readonly transport: InvokeTransport;
  readonly nextRequestId: () => string;
}

export function createTauriOperationPort(
  options: TauriOperationPortOptions,
): OperationPort {
  const events = isEventTransport(options.transport)
    ? options.transport
    : undefined;
  return {
    snapshot: (query) =>
      options.transport.invoke(OPERATION_SNAPSHOT_COMMAND, { query }),
    mutate: (command) =>
      options.transport.invoke(OPERATION_MUTATE_COMMAND, { command }),
    cancel: (command) =>
      options.transport.invoke(OPERATION_CANCEL_COMMAND, { command }),
    listen:
      events === undefined
        ? undefined
        : (listener) => listenChanged(events, listener),
    nextRequestId: options.nextRequestId,
  };
}

function listenChanged(
  transport: EventTransport,
  listener: (event: unknown) => void,
) {
  return transport.listen(OPERATION_CHANGED_EVENT, listener);
}
