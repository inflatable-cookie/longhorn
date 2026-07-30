import type { BridgeHelloRequest } from "./generated/protocol.ts";
import type { BridgeOperationDescriptor } from "./operation.ts";
import { BridgeHostRouter } from "./router.ts";

export type BridgeSerializationPhase =
  | "negotiation_request"
  | "negotiation_reply"
  | "operation_request"
  | "operation_reply"
  | "snapshot"
  | "event";

export class BridgeSerializationError extends Error {
  readonly phase: BridgeSerializationPhase;
  override readonly cause: unknown;

  constructor(phase: BridgeSerializationPhase, cause: unknown) {
    super(`bridge serialization failed: ${phase}`, { cause });
    this.name = "BridgeSerializationError";
    this.phase = phase;
    this.cause = cause;
  }
}

export class SerializedLoopbackBridgeAdapter {
  readonly #router: BridgeHostRouter;

  constructor(router: BridgeHostRouter) {
    this.#router = router;
  }

  async negotiate(request: BridgeHelloRequest): Promise<unknown> {
    const encodedRequest = serialize(
      request,
      "negotiation_request",
    );
    const reply = await this.#router.negotiate(JSON.parse(encodedRequest));
    return deserialize(
      serialize(reply, "negotiation_reply"),
      "negotiation_reply",
    );
  }

  async invoke<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): Promise<unknown> {
    const encodedRequest = serialize(request, "operation_request");
    const reply = await this.#router.invoke(
      operation.route,
      JSON.parse(encodedRequest),
    );
    return deserialize(
      serialize(reply, "operation_reply"),
      "operation_reply",
    );
  }
}

export function serialize(
  value: unknown,
  phase: BridgeSerializationPhase,
): string {
  try {
    const encoded = JSON.stringify(value);
    if (encoded === undefined) {
      throw new TypeError("value has no JSON representation");
    }
    return encoded;
  } catch (error) {
    throw new BridgeSerializationError(phase, error);
  }
}

export function deserialize(
  value: string,
  phase: BridgeSerializationPhase,
): unknown {
  try {
    return JSON.parse(value);
  } catch (error) {
    throw new BridgeSerializationError(phase, error);
  }
}
