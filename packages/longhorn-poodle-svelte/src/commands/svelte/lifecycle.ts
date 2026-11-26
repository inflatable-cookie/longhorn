import { useSessionLifecycle } from "../../bindings/mount.ts";
import type { CommandSession } from "./session.svelte.ts";

export function useCommandSession(getSession: () => CommandSession): void {
  useSessionLifecycle(getSession);
}
