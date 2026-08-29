import { onMount } from "svelte";

import type { NotificationSession } from "./session.svelte.ts";

export function useNotificationSession(getSession: () => NotificationSession): void {
  onMount(() => {
    const session = getSession();
    void session.start().catch(() => undefined);
    return () => { void session.stop(); };
  });
}
