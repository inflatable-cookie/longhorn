import { BridgeConnectionRuntime } from "@inflatable-cookie/longhorn/bridge";
import type { BridgeRuntimeClock } from "@inflatable-cookie/longhorn/bridge";

import { receipt } from "./common.ts";

export function lifecycleArtifactTrace() {
  const clock: BridgeRuntimeClock & { value: number } = {
    value: 100,
    now() {
      return this.value;
    },
  };
  const runtime = new BridgeConnectionRuntime(
    clock,
    { delay: (_class, attempt) => attempt * 25 },
    1,
  );
  const states = [
    runtime.connect().current.state,
    runtime.transportReady().current.state,
    runtime.acceptNegotiation(receipt("reconnecting-lifecycle", "localFirst"), []).current.state,
  ];
  const reconnect = runtime.reconnect("afterReconnect");
  const reconnectState = reconnect.current.state;
  const notBefore = reconnect.reconnect?.notBefore;
  if (notBefore === undefined) {
    throw new Error("reconnect scheduled no retry time");
  }
  clock.value = notBefore;
  const transportState = runtime.transportReady().current.state;
  const negotiatedState = runtime.acceptNegotiation(
    receipt(
      "reconnecting-lifecycle",
      "remote",
      "session:fixture-reconnecting-lifecycle-reconnected",
    ),
    [],
  ).current.state;
  states.push(reconnectState, transportState, negotiatedState, runtime.close().current.state);
  return {
    states,
    reconnectAttempt: reconnect.reconnect?.attempt,
    sessionInvalidated: reconnect.sessionId === null,
  } as const;
}
