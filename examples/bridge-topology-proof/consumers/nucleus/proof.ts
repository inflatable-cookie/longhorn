import { runNucleusTrace } from "../../nucleus.ts";

const trace = await runNucleusTrace();
if (
  !trace.hostConnectionSeparate ||
  !trace.domainIdentityStable ||
  trace.capabilityDoesNotGrantWrite !== "write_denied" ||
  trace.capabilityDoesNotGrantExecution !== "execution_denied" ||
  !trace.executionParity ||
  !trace.queryParity
) {
  throw new Error("Nucleus artifact trace violated authority policy");
}
console.log(JSON.stringify({ trace }));
