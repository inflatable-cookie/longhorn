import type { OperationPort } from "./ports.ts";

export interface DirectOperationHandlers {
  readonly snapshot: OperationPort["snapshot"];
  readonly mutate: OperationPort["mutate"];
  readonly cancel: OperationPort["cancel"];
  readonly listen?: OperationPort["listen"];
  readonly nextRequestId: OperationPort["nextRequestId"];
}

export function createDirectOperationPort(
  handlers: DirectOperationHandlers,
): OperationPort {
  return { ...handlers };
}
