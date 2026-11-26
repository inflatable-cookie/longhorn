import type { Unlisten } from "./transport.ts";

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
 * not skip the rest. Results include any failures and follow registration order.
 */
export function disposeAll(
  unlisten: ReadonlyArray<() => void | Promise<void>>,
): Promise<Array<PromiseSettledResult<void>>> {
  return Promise.allSettled(unlisten.map((dispose) => attempt(dispose)));
}
/** Holds one unlisten and shares its result between competing cleanup paths. */
export class ListenerDisposal {
  unlisten: Unlisten | undefined;
  #cleanup: Promise<void> | undefined;

  close(): Promise<void> {
    const unlisten = this.unlisten;
    if (unlisten !== undefined) {
      this.unlisten = undefined;
      this.#cleanup = Promise.resolve().then(() => unlisten());
    }
    return this.#cleanup ?? Promise.resolve();
  }
}
