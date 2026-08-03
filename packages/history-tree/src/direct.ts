import type { ForkHistoryPort } from "./ports.ts";
export type DirectForkHistoryHandlers = ForkHistoryPort;
export function createDirectForkHistoryPort(handlers: DirectForkHistoryHandlers): ForkHistoryPort { return handlers; }
