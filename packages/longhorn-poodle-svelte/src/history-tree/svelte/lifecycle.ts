import { onMount } from "svelte";
import type { ForkHistorySession } from "./session.svelte.ts";
export function useForkHistorySession(getSession: () => ForkHistorySession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
