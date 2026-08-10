import { runJobsAndServiceFailureTrace } from "../../jobs-and-service-failure.ts";

const trace = await runJobsAndServiceFailureTrace();
if (
  !trace.cancellationParity ||
  !trace.jobParity ||
  trace.serviceState !== "failed" ||
  !trace.localDomainsAvailableAfterServiceFailure
) {
  throw new Error("Jobs-and-service-failure artifact trace violated job/service policy");
}
console.log(JSON.stringify({ trace }));
