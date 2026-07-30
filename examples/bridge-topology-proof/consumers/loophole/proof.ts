import { lifecycleArtifactTrace } from "../../lifecycle.ts";
import { runLoopholeTrace } from "../../loophole.ts";

const trace = await runLoopholeTrace();
const lifecycle = lifecycleArtifactTrace();
if (
  !trace.authorityStable ||
  !trace.queryParity ||
  lifecycle.states.join(",") !==
    "connecting,negotiating,ready,reconnecting,negotiating,ready,closed" ||
  !lifecycle.sessionInvalidated
) {
  throw new Error("Loophole artifact trace violated topology policy");
}
console.log(JSON.stringify({ trace, lifecycle }));
