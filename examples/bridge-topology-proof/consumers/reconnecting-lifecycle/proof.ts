import { lifecycleArtifactTrace } from "../../lifecycle.ts";
import { runReconnectingLifecycleTrace } from "../../reconnecting-lifecycle.ts";

const trace = await runReconnectingLifecycleTrace();
const lifecycle = lifecycleArtifactTrace();
if (
  !trace.authorityStable ||
  !trace.queryParity ||
  lifecycle.states.join(",") !==
    "connecting,negotiating,ready,reconnecting,negotiating,ready,closed" ||
  !lifecycle.sessionInvalidated
) {
  throw new Error("Reconnecting-lifecycle artifact trace violated topology policy");
}
console.log(JSON.stringify({ trace, lifecycle }));
