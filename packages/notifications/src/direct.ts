import type { NotificationPort } from "./ports.ts";

export interface DirectNotificationHandlers {
  readonly snapshot: NotificationPort["snapshot"];
  readonly mutate: NotificationPort["mutate"];
  readonly listen?: NotificationPort["listen"];
  readonly nextRequestId: NotificationPort["nextRequestId"];
}

export function createDirectNotificationPort(handlers: DirectNotificationHandlers): NotificationPort {
  return { ...handlers };
}
