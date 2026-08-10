import { BridgeServiceRuntime } from "@inflatable-cookie/longhorn/bridge/supervision";

import {
  authorityProjection,
  queryParity,
  receipt,
  sameValues,
} from "./common.ts";

export async function runReconnectingLifecycleTrace() {
  const embedded = receipt("reconnecting-lifecycle", "localFirst");
  const remote = receipt(
    "reconnecting-lifecycle",
    "remote",
    "session:fixture-reconnecting-lifecycle-remote",
  );
  const supervisor = new BridgeServiceRuntime("externalRemote");
  const attach = await supervisor.execute(
    { perform: () => "accepted" },
    "attach",
  );
  const query = await queryParity("reconnecting-lifecycle");

  return {
    hostForms: [embedded.host.form, remote.host.form],
    hostInstancesDiffer:
      embedded.host.hostInstanceId !== remote.host.hostInstanceId,
    sessionsDiffer: embedded.sessionId !== remote.sessionId,
    authorityStable:
      JSON.stringify(authorityProjection(embedded)) ===
      JSON.stringify(authorityProjection(remote)),
    remoteLifecycle: attach.current,
    queryParity: sameValues(query.traces),
  } as const;
}
