import { runJetstreamTrace } from "../../jetstream.ts";

const trace = await runJetstreamTrace();
if (
  !trace.listenerFirst ||
  !trace.gapResync ||
  !trace.adapterParity ||
  !trace.queryParity
) {
  throw new Error("Jetstream artifact trace violated stream policy");
}
console.log(JSON.stringify({ trace }));
