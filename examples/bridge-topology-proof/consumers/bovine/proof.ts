import { runBovineTrace } from "../../bovine.ts";
import { protocolArtifactTrace } from "../../protocol-artifact.ts";

const trace = await runBovineTrace();
if (
  !trace.adapterParity ||
  trace.eventsResolved ||
  trace.serviceResolved
) {
  throw new Error("Bovine artifact trace violated query-only policy");
}
console.log(JSON.stringify({ trace, protocol: protocolArtifactTrace() }));
