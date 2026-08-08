import {
  isEventTransport,
  type EventTransport,
  type InvokeTransport,
} from "@inflatable-cookie/longhorn/core";

import type { NativeContentPort } from "@inflatable-cookie/longhorn/native-content";

export const NATIVE_CONTENT_CONNECT_COMMAND =
  "longhorn_native_content_connect";
export const NATIVE_CONTENT_SNAPSHOT_COMMAND =
  "longhorn_native_content_snapshot";
export const NATIVE_CONTENT_UPDATE_DESIRED_COMMAND =
  "longhorn_native_content_update_desired";
export const NATIVE_CONTENT_DECIDE_SIZE_COMMAND =
  "longhorn_native_content_decide_size";
export const NATIVE_CONTENT_CHANGED_EVENT =
  "longhorn://native-content/changed";

export interface TauriNativeContentPortOptions {
  readonly transport: InvokeTransport;
  readonly nextRequestId: () => string;
}

export function createTauriNativeContentPort(
  options: TauriNativeContentPortOptions,
): NativeContentPort {
  const events = isEventTransport(options.transport)
    ? options.transport
    : undefined;
  return {
    connect: (request) =>
      options.transport.invoke(NATIVE_CONTENT_CONNECT_COMMAND, { request }),
    snapshot: (request) =>
      options.transport.invoke(NATIVE_CONTENT_SNAPSHOT_COMMAND, { request }),
    updateDesired: (request) =>
      options.transport.invoke(NATIVE_CONTENT_UPDATE_DESIRED_COMMAND, {
        request,
      }),
    decideContentSize: (request) =>
      options.transport.invoke(NATIVE_CONTENT_DECIDE_SIZE_COMMAND, { request }),
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
  return transport.listen(NATIVE_CONTENT_CHANGED_EVENT, listener);
}
