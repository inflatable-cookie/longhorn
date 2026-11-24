import type { CommandUnlisten } from "./ports.ts";

/**
 * Runs `run` inside a promise so a synchronous throw becomes a rejection that
 * the caller can settle alongside the others, instead of abandoning them.
 */
export function attempt<Value>(run: () => Value | PromiseLike<Value>): Promise<Value> {
  return new Promise<Value>((resolve) => {
    resolve(run());
  });
}

/**
 * Disposes every registration. One unlisten that throws, sync or async, does
 * not skip the rest, and its failure is settled rather than raised.
 */
export async function disposeAll(unlisten: ReadonlyArray<CommandUnlisten>): Promise<void> {
  await Promise.allSettled(unlisten.map((dispose) => attempt(dispose)));
}
