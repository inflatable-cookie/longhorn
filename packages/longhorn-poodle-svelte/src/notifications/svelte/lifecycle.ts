import { useSessionLifecycle } from "../../bindings/mount.ts";
import type { NotificationSession } from "./session.svelte.ts";

export function useNotificationSession(getSession: () => NotificationSession): void {
  useSessionLifecycle(getSession);
}
