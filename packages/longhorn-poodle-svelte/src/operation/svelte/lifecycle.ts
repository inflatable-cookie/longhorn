import { onMount } from "svelte";

import { reportFailure } from "../../failures.ts";

import type { OperationSession } from "./session.svelte.ts";

export function useOperationSession(getSession: () => OperationSession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop().catch(reportFailure);
    };
  });
}
