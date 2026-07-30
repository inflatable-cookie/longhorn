import { runSoundcheckTrace } from "../../soundcheck.ts";

const trace = await runSoundcheckTrace();
if (
  !trace.cancellationParity ||
  !trace.jobParity ||
  trace.serviceState !== "failed" ||
  !trace.localDomainsAvailableAfterServiceFailure
) {
  throw new Error("Soundcheck artifact trace violated job/service policy");
}
console.log(JSON.stringify({ trace }));
