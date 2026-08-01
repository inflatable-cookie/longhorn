import { onMount } from "svelte";

import type { NativeContentSession } from "./session.svelte.ts";

export function useNativeContentSession(session: NativeContentSession): void {
  onMount(() => {
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
