import { onMount } from "svelte";

import { reportFailure } from "../../failures.ts";
import type { ForkHistorySession } from "./session.svelte.ts";
export function useForkHistorySession(getSession: () => ForkHistorySession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop().catch(reportFailure);
    };
  });
}
