import { useSessionLifecycle } from "../bindings/mount.ts";
import type { NativeContentSession } from "./session.svelte.ts";

export function useNativeContentSession(getSession: () => NativeContentSession): void {
  useSessionLifecycle(getSession);
}
