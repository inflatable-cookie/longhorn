import type { NativeContentPort } from "./ports.ts";

export interface DirectNativeContentHandlers {
  readonly connect: NativeContentPort["connect"];
  readonly snapshot: NativeContentPort["snapshot"];
  readonly updateDesired: NativeContentPort["updateDesired"];
  readonly decideContentSize: NativeContentPort["decideContentSize"];
  readonly listen?: NativeContentPort["listen"];
  readonly nextRequestId: NativeContentPort["nextRequestId"];
}

export function createDirectNativeContentPort(
  handlers: DirectNativeContentHandlers,
): NativeContentPort {
  return { ...handlers };
}
