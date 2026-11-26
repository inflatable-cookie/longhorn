import { useSessionLifecycle } from "../../bindings/mount.ts";
import type { ForkHistorySession } from "./session.svelte.ts";

export function useForkHistorySession(getSession: () => ForkHistorySession): void {
  useSessionLifecycle(getSession);
}
