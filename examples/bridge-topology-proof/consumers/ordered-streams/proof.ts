import { runOrderedStreamsTrace } from "../../ordered-streams.ts";

const trace = await runOrderedStreamsTrace();
if (
  !trace.listenerFirst ||
  !trace.gapResync ||
  !trace.adapterParity ||
  !trace.queryParity
) {
  throw new Error("Ordered-streams artifact trace violated stream policy");
}
console.log(JSON.stringify({ trace }));
