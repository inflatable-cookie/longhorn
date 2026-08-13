import type { UpdatePort } from "./ports.ts";
export type DirectUpdateHandlers = UpdatePort;
export function createDirectUpdatePort(handlers: DirectUpdateHandlers): UpdatePort { return handlers; }
