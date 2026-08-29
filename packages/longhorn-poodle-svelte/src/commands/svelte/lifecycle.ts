import { onMount } from "svelte";

import type { CommandSession } from "./session.svelte.ts";

export function useCommandSession(getSession: () => CommandSession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
