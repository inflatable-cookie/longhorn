import { onMount } from "svelte";

import { reportFailure } from "../failures.ts";
import type { StartableClientState } from "./lifecycle.svelte.ts";

export function useSessionLifecycle(
  getSession: () => StartableClientState,
  onStartFailure: (error: unknown) => void = () => undefined,
  onStopFailure: (error: unknown) => void = reportFailure,
): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(onStartFailure);
    return () => {
      void session.stop().catch(onStopFailure);
    };
  });
}
