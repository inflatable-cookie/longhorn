import { queryParity, sameValues } from "./common.ts";

export async function runMinimalShellTrace() {
  const query = await queryParity("minimal-shell");
  return {
    topology: "tauriLocal",
    queryOnly: true,
    adapterParity: sameValues(query.traces),
    query,
    eventsResolved: false,
    serviceResolved: false,
  } as const;
}
