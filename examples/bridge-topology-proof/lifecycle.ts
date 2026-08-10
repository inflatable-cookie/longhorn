import {
  BridgeConnectionRuntime,
  type BridgeRuntimeClock,
} from "@inflatable-cookie/longhorn/bridge";

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
    runtime.acceptNegotiation(receipt("reconnecting-lifecycle", "localFirst"), []).current
      .state,
  ];
  const reconnect = runtime.reconnect("afterReconnect");
  states.push(reconnect.current.state);
  clock.value = reconnect.reconnect!.notBefore;
  states.push(runtime.transportReady().current.state);
  states.push(
    runtime.acceptNegotiation(
      receipt(
        "reconnecting-lifecycle",
        "remote",
        "session:fixture-reconnecting-lifecycle-reconnected",
      ),
      [],
    ).current.state,
  );
  states.push(runtime.close().current.state);
  return {
    states,
    reconnectAttempt: reconnect.reconnect?.attempt,
    sessionInvalidated: reconnect.sessionId === null,
  } as const;
}
