import { onMount } from "svelte";

import type { NotificationSession } from "./session.svelte.ts";

export function useNotificationSession(session: NotificationSession): void {
  onMount(() => {
    void session.start().catch(() => undefined);
    return () => { void session.stop(); };
  });
}
