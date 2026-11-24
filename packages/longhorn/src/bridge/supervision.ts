import type {
  BridgeCredentialRef,
  BridgeServiceAction,
  BridgeServiceOutcome,
  BridgeServiceOwnership,
  BridgeServiceRequest,
  BridgeServiceState,
  BridgeServiceTransitionReceipt,
} from "./generated/protocol.ts";
import { BRIDGE_SERVICE_FAILURE_CODES, BRIDGE_SERVICE_OWNERSHIPS } from "./generated/protocol.ts";
import { opaqueId } from "./validation/base.ts";

export interface BridgeServiceSupervisorPort {
  perform(request: BridgeServiceRequest): Promise<unknown> | unknown;
}

/**
 * Monotonic service incarnation.
 *
 * One incarnation covers one service attempt. It advances only when a commit
 * starts an attempt (spawn, attach, restart or reconnect), never for a
 * readiness observation of the running attempt and never for the attempt's
 * end.
 */
export type BridgeServiceIncarnation = number;

/**
 * One admitted supervision action bound to the incarnation that admitted it.
 *
 * A runtime-local token, not a wire shape: `begin` returns it and `commit`
 * accepts its observation only while that incarnation is still current.
 */
export interface BridgeServiceAdmission {
  readonly incarnation: BridgeServiceIncarnation;
  readonly action: BridgeServiceAction;
}

export class BridgeSupervisionError extends Error {
  readonly code:
    | "lifecycle_not_owned"
    | "invalid_ownership"
    | "invalid_transition"
    | "invalid_observation"
    | "stale_observation"
    | "adapter_failed";

  constructor(code: BridgeSupervisionError["code"]) {
    super(`bridge supervision: ${code}`);
    this.name = "BridgeSupervisionError";
    this.code = code;
  }
}

export class BridgeServiceRuntime {
  readonly #ownership: BridgeServiceOwnership;
  #state: BridgeServiceState = "absent";
  #generation = 0;
  #incarnation = 0;

  constructor(ownership: BridgeServiceOwnership) {
    if (!BRIDGE_SERVICE_OWNERSHIPS.includes(ownership)) {
      throw new BridgeSupervisionError("invalid_ownership");
    }
    this.#ownership = ownership;
  }

  get state(): BridgeServiceState {
    return this.#state;
  }

  async execute(
    port: BridgeServiceSupervisorPort,
    action: BridgeServiceAction,
    credentialRef?: BridgeCredentialRef,
  ): Promise<BridgeServiceTransitionReceipt> {
    const admission = this.begin(action);
    const request: BridgeServiceRequest = {
      action,
      credentialRef: credentialRef === undefined ? null : opaqueId(credentialRef),
    };
    let rawOutcome: unknown;
    try {
      rawOutcome = await port.perform(request);
    } catch {
      throw new BridgeSupervisionError("adapter_failed");
    }
    return this.commit(admission, rawOutcome);
  }

  /**
   * Admits one action whose observation arrives after the call returns.
   *
   * The returned admission carries the service incarnation that admitted it, so
   * the observation must be committed through `commit`. A commit that starts a
   * new attempt in between supersedes the admission and the delayed observation
   * is rejected instead of being reported for the later service incarnation.
   * Readiness observations of one attempt share its incarnation, so overlapping
   * probes of that attempt all stay committable.
   */
  begin(action: BridgeServiceAction): BridgeServiceAdmission {
    this.#admit(action);
    return { incarnation: this.#incarnation, action };
  }

  /**
   * Commits the observation for an action admitted by `begin`.
   *
   * Rejects the observation when a commit started a new service attempt after
   * admission, or when the action is no longer valid in the current state.
   * Neither a readiness observation nor the end of an attempt changes the
   * incarnation, so a probe and a shutdown admitted together both land.
   */
  commit(admission: BridgeServiceAdmission, value: unknown): BridgeServiceTransitionReceipt {
    if (admission.incarnation !== this.#incarnation) {
      throw new BridgeSupervisionError("stale_observation");
    }
    this.#admit(admission.action);
    return this.#apply(admission.action, value);
  }

  /**
   * Commits an adapter observation that is current when the call is made.
   *
   * This admits and commits in one step, so it cannot carry an observation that
   * started before the call. Use `begin` and `commit` for one that completes
   * later.
   */
  observe(action: BridgeServiceAction, value: unknown): BridgeServiceTransitionReceipt {
    return this.commit(this.begin(action), value);
  }

  #apply(action: BridgeServiceAction, value: unknown): BridgeServiceTransitionReceipt {
    const outcome = parseOutcome(value);
    const current = transitionState(action, outcome);
    const generation = this.#generation + 1;
    const incarnation = startsIncarnation(current) ? this.#incarnation + 1 : this.#incarnation;
    if (!Number.isSafeInteger(generation) || !Number.isSafeInteger(incarnation)) {
      throw new BridgeSupervisionError("invalid_transition");
    }
    const receipt = {
      generation,
      ownership: this.#ownership,
      action,
      previous: this.#state,
      current,
      outcome,
    };
    this.#generation = generation;
    this.#incarnation = incarnation;
    this.#state = current;
    return receipt;
  }

  #admit(action: BridgeServiceAction): void {
    const owned = this.#ownership === "ownedLocal";
    let admitted = false;
    switch (action) {
      case "spawn":
        admitted = owned && ["absent", "stopped", "failed"].includes(this.#state);
        break;
      case "attach":
        admitted = !owned && ["absent", "stopped", "failed"].includes(this.#state);
        break;
      case "checkReadiness":
        admitted = [
          "starting",
          "attaching",
          "awaitingReadiness",
          "restarting",
          "reconnecting",
          "ready",
        ].includes(this.#state);
        break;
      case "restart":
        admitted = owned && ["ready", "failed"].includes(this.#state);
        break;
      case "reconnect":
        admitted = ["ready", "failed", "stopped"].includes(this.#state);
        break;
      case "shutdown":
        admitted = owned && !["absent", "stopped"].includes(this.#state);
        break;
    }
    if (admitted) {
      return;
    }
    if (!owned && ["spawn", "restart", "shutdown"].includes(action)) {
      throw new BridgeSupervisionError("lifecycle_not_owned");
    }
    throw new BridgeSupervisionError("invalid_transition");
  }
}

function transitionState(
  action: BridgeServiceAction,
  outcome: BridgeServiceOutcome,
): BridgeServiceState {
  if (typeof outcome === "object") {
    return "failed";
  }
  if (action === "checkReadiness" && outcome === "ready") {
    return "ready";
  }
  if (action === "checkReadiness" && outcome === "notReady") {
    return "awaitingReadiness";
  }
  if (action === "shutdown" && outcome === "stopped") {
    return "stopped";
  }
  if (outcome === "accepted") {
    const accepted: Partial<Record<BridgeServiceAction, BridgeServiceState>> = {
      spawn: "starting",
      attach: "attaching",
      restart: "restarting",
      reconnect: "reconnecting",
    };
    const state = accepted[action];
    if (state !== undefined) {
      return state;
    }
  }
  throw new BridgeSupervisionError("invalid_observation");
}

function startsIncarnation(state: BridgeServiceState): boolean {
  return ["starting", "attaching", "restarting", "reconnecting"].includes(state);
}

function parseOutcome(value: unknown): BridgeServiceOutcome {
  if (value === "accepted" || value === "ready" || value === "notReady" || value === "stopped") {
    return value;
  }
  if (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.keys(value).length === 1 &&
    "failed" in value
  ) {
    const failed = BRIDGE_SERVICE_FAILURE_CODES.find((code) => code === value.failed);
    if (failed !== undefined) {
      return { failed };
    }
  }
  throw new BridgeSupervisionError("invalid_observation");
}
