import { onMount } from "svelte";

import type { HistorySession } from "./session.svelte.ts";

export function useHistorySession(getSession: () => HistorySession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
