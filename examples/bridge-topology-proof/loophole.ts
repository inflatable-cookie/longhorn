import { BridgeServiceRuntime } from "@inflatable-cookie/longhorn/bridge/supervision";

import {
  authorityProjection,
  queryParity,
  receipt,
  sameValues,
} from "./common.ts";

export async function runLoopholeTrace() {
  const embedded = receipt("loophole", "localFirst");
  const remote = receipt(
    "loophole",
    "remote",
    "session:fixture-loophole-remote",
  );
  const supervisor = new BridgeServiceRuntime("externalRemote");
  const attach = await supervisor.execute(
    { perform: () => "accepted" },
    "attach",
  );
  const query = await queryParity("loophole");

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
