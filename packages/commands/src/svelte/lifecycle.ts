import { onMount } from "svelte";

import type { CommandSession } from "./session.svelte.ts";

export function useCommandSession(session: CommandSession): void {
  onMount(() => {
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
