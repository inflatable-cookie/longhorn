import type { HistoryPort } from "./ports.ts";

export interface DirectHistoryHandlers {
  readonly snapshot: HistoryPort["snapshot"];
  readonly page: HistoryPort["page"];
  readonly navigate: HistoryPort["navigate"];
  readonly listen?: HistoryPort["listen"];
  readonly nextPlanId: HistoryPort["nextPlanId"];
}

export function createDirectHistoryPort(
  handlers: DirectHistoryHandlers,
): HistoryPort {
  return {
    snapshot: handlers.snapshot,
    page: handlers.page,
    navigate: handlers.navigate,
    listen: handlers.listen,
    nextPlanId: handlers.nextPlanId,
  };
}
