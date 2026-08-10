import type {
  BridgeCredentialRef,
  BridgeServiceAction,
  BridgeServiceFailureCode,
  BridgeServiceOutcome,
  BridgeServiceOwnership,
  BridgeServiceRequest,
  BridgeServiceState,
  BridgeServiceTransitionReceipt,
} from "./generated/protocol.ts";
import { BRIDGE_SERVICE_OWNERSHIPS } from "./generated/protocol.ts";
import { opaqueId } from "./validation/base.ts";

export interface BridgeServiceSupervisorPort {
  perform(
    request: BridgeServiceRequest,
  ): Promise<unknown> | unknown;
}

export class BridgeSupervisionError extends Error {
  readonly code:
    | "lifecycle_not_owned"
    | "invalid_transition"
    | "invalid_observation"
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

  constructor(ownership: BridgeServiceOwnership) {
    if (!BRIDGE_SERVICE_OWNERSHIPS.includes(ownership)) {
      throw new BridgeSupervisionError("invalid_transition");
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
    this.#admit(action);
    const request: BridgeServiceRequest = {
      action,
      credentialRef: credentialRef === undefined
        ? null
        : opaqueId(credentialRef),
    };
    let rawOutcome: unknown;
    try {
      rawOutcome = await port.perform(request);
    } catch {
      throw new BridgeSupervisionError("adapter_failed");
    }
    return this.observe(action, rawOutcome);
  }

  observe(
    action: BridgeServiceAction,
    value: unknown,
  ): BridgeServiceTransitionReceipt {
    this.#admit(action);
    const outcome = parseOutcome(value);
    const current = transitionState(action, outcome);
    const generation = this.#generation + 1;
    if (!Number.isSafeInteger(generation)) {
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
    this.#state = current;
    return receipt;
  }

  #admit(action: BridgeServiceAction): void {
    const owned = this.#ownership === "ownedLocal";
    let admitted = false;
    switch (action) {
      case "spawn":
        admitted = owned &&
          ["absent", "stopped", "failed"].includes(this.#state);
        break;
      case "attach":
        admitted = !owned &&
          ["absent", "stopped", "failed"].includes(this.#state);
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
    if (
      !owned &&
      ["spawn", "restart", "shutdown"].includes(action)
    ) {
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

function parseOutcome(value: unknown): BridgeServiceOutcome {
  if (
    value === "accepted" ||
    value === "ready" ||
    value === "notReady" ||
    value === "stopped"
  ) {
    return value;
  }
  if (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.keys(value).length === 1 &&
    "failed" in value &&
    isFailureCode(value.failed)
  ) {
    return { failed: value.failed };
  }
  throw new BridgeSupervisionError("invalid_observation");
}

function isFailureCode(value: unknown): value is BridgeServiceFailureCode {
  return typeof value === "string" &&
    [
      "spawnFailed",
      "attachFailed",
      "readinessFailed",
      "serviceExited",
      "restartFailed",
      "reconnectFailed",
      "shutdownFailed",
    ].includes(value);
}
