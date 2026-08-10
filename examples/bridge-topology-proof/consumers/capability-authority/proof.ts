import { runCapabilityAuthorityTrace } from "../../capability-authority.ts";

const trace = await runCapabilityAuthorityTrace();
if (
  !trace.hostConnectionSeparate ||
  !trace.domainIdentityStable ||
  trace.capabilityDoesNotGrantWrite !== "write_denied" ||
  trace.capabilityDoesNotGrantExecution !== "execution_denied" ||
  !trace.executionParity ||
  !trace.queryParity
) {
  throw new Error("Capability-authority artifact trace violated authority policy");
}
console.log(JSON.stringify({ trace }));
