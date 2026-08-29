import { onMount } from "svelte";

import type { NativeContentSession } from "./session.svelte.ts";

export function useNativeContentSession(getSession: () => NativeContentSession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => {
      void session.stop();
    };
  });
}
