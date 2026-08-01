import { onMount } from "svelte";

import type { HistorySession } from "./session.svelte.ts";

export function useHistorySession(session: HistorySession): void {
  onMount(() => {
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
