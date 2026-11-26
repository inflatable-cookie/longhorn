import { useSessionLifecycle } from "../../bindings/mount.ts";
import type { OperationSession } from "./session.svelte.ts";

export function useOperationSession(getSession: () => OperationSession): void {
  useSessionLifecycle(getSession);
}
