import { queryParity, sameValues } from "./common.ts";

export async function runBovineTrace() {
  const query = await queryParity("bovine");
  return {
    topology: "tauriLocal",
    queryOnly: true,
    adapterParity: sameValues(query.traces),
    query,
    eventsResolved: false,
    serviceResolved: false,
  } as const;
}
