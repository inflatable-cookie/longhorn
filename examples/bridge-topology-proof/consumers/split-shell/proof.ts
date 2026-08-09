import { runSplit-shellTrace } from "../../split-shell.ts";
import { protocolArtifactTrace } from "../../protocol-artifact.ts";

const trace = await runSplit-shellTrace();
if (
  !trace.adapterParity ||
  trace.eventsResolved ||
  trace.serviceResolved
) {
  throw new Error("Split-shell artifact trace violated query-only policy");
}
console.log(JSON.stringify({ trace, protocol: protocolArtifactTrace() }));
