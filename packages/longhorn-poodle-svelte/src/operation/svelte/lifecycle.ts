import { onMount } from "svelte";

import type { OperationSession } from "./session.svelte.ts";

export function useOperationSession(session: OperationSession): void {
  onMount(() => {
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
