import { runMinimalShellTrace } from "../../minimal-shell.ts";
import { protocolArtifactTrace } from "../../protocol-artifact.ts";

const trace = await runMinimalShellTrace();
if (
  !trace.adapterParity ||
  trace.eventsResolved ||
  trace.serviceResolved
) {
  throw new Error("Minimal shell artifact trace violated query-only policy");
}
console.log(JSON.stringify({ trace, protocol: protocolArtifactTrace() }));
