import { useSessionLifecycle } from "../../bindings/mount.ts";
import type { HistorySession } from "./session.svelte.ts";

export function useHistorySession(getSession: () => HistorySession): void {
  useSessionLifecycle(getSession);
}
