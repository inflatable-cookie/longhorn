import { isEventTransport, type EventTransport, type InvokeTransport } from "@longhorn/core";
import type { ForkHistoryPort } from "./ports.ts";
export const FORK_HISTORY_SNAPSHOT_COMMAND = "longhorn_history_tree_snapshot";
export const FORK_HISTORY_PATH_COMMAND = "longhorn_history_tree_path";
export const FORK_HISTORY_BRANCHES_COMMAND = "longhorn_history_tree_branches";
export const FORK_HISTORY_NAVIGATE_COMMAND = "longhorn_history_tree_navigate";
export const FORK_HISTORY_CHANGED_EVENT = "longhorn://history-tree/changed";
export function createTauriForkHistoryPort(options: { readonly transport: InvokeTransport; readonly nextPlanId: () => string }): ForkHistoryPort {
  const events = isEventTransport(options.transport) ? options.transport : undefined;
  return {
    snapshot: () => options.transport.invoke(FORK_HISTORY_SNAPSHOT_COMMAND, {}),
    path: (command) => options.transport.invoke(FORK_HISTORY_PATH_COMMAND, { command }),
    branches: (command) => options.transport.invoke(FORK_HISTORY_BRANCHES_COMMAND, { command }),
    navigate: (command) => options.transport.invoke(FORK_HISTORY_NAVIGATE_COMMAND, { command }),
    listen: events === undefined ? undefined : (listener) => listen(events, listener),
    nextPlanId: options.nextPlanId,
  };
}
function listen(events: EventTransport, listener: (event: unknown) => void) { return events.listen(FORK_HISTORY_CHANGED_EVENT, listener); }
