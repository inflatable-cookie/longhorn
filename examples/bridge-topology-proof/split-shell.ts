import { queryParity, sameValues } from "./common.ts";

export async function runSplit-shellTrace() {
  const query = await queryParity("split-shell");
  return {
    topology: "tauriLocal",
    queryOnly: true,
    adapterParity: sameValues(query.traces),
    query,
    eventsResolved: false,
    serviceResolved: false,
  } as const;
}
