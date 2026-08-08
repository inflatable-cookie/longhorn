import type {
  BridgeQueryRetryDecision,
  BridgeReconnectSchedule,
  BridgeRetryClass,
} from "../generated/protocol.ts";
import {
  checkedDeadline,
  checkedMonotonic,
  checkedRetryClass,
  checkedRetryLimit,
  type BridgeRuntimeBackoff,
  type BridgeRuntimeClock,
} from "./policy.ts";

export class BridgeQueryRetryRuntime {
  readonly #clock: BridgeRuntimeClock;
  readonly #backoff: BridgeRuntimeBackoff;
  readonly #limit: number;
  #scheduled = 0;

  constructor(
    clock: BridgeRuntimeClock,
    backoff: BridgeRuntimeBackoff,
    limit: number,
  ) {
    this.#clock = clock;
    this.#backoff = backoff;
    this.#limit = checkedRetryLimit(limit);
  }

  schedule(
    decision: BridgeQueryRetryDecision,
    retryClass: BridgeRetryClass,
  ): BridgeReconnectSchedule | undefined {
    retryClass = checkedRetryClass(retryClass);
    if (
      decision !== "retry" ||
      retryClass === "never" ||
      this.#scheduled >= this.#limit
    ) {
      return undefined;
    }
    const attempt = this.#scheduled + 1;
    const at = checkedMonotonic(this.#clock.now());
    const schedule = {
      attempt,
      retryClass,
      notBefore: checkedDeadline(
        at,
        this.#backoff.delay(retryClass, attempt),
      ),
    };
    this.#scheduled = attempt;
    return schedule;
  }

  reset(): void {
    this.#scheduled = 0;
  }
}
