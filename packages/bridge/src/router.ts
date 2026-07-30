import type {
  BridgeHelloRequest,
  BridgeNegotiationReceipt,
} from "./generated/protocol.ts";
import {
  parseBridgeHelloRequest,
  parseBridgeNegotiationReceipt,
} from "./compatibility.ts";
import type { BridgeOperationDescriptor } from "./operation.ts";

type BridgeOperationHandler<Request, Reply> = (
  request: Request,
) => Reply | Promise<Reply>;

interface RegisteredOperation {
  readonly descriptor: BridgeOperationDescriptor<unknown, unknown>;
  readonly handler: BridgeOperationHandler<unknown, unknown>;
}

export class BridgeHostRouter {
  readonly #negotiate: (
    request: BridgeHelloRequest,
  ) => BridgeNegotiationReceipt | Promise<BridgeNegotiationReceipt>;
  readonly #operations = new Map<string, RegisteredOperation>();

  constructor(
    negotiate: (
      request: BridgeHelloRequest,
    ) => BridgeNegotiationReceipt | Promise<BridgeNegotiationReceipt>,
  ) {
    this.#negotiate = negotiate;
  }

  register<Request, Reply>(
    descriptor: BridgeOperationDescriptor<Request, Reply>,
    handler: BridgeOperationHandler<Request, Reply>,
  ): () => void {
    if (this.#operations.has(descriptor.route)) {
      throw new Error(`duplicate bridge operation route: ${descriptor.route}`);
    }
    const registration: RegisteredOperation = {
      descriptor: descriptor as BridgeOperationDescriptor<unknown, unknown>,
      handler: handler as BridgeOperationHandler<unknown, unknown>,
    };
    this.#operations.set(descriptor.route, registration);
    return () => {
      if (this.#operations.get(descriptor.route) === registration) {
        this.#operations.delete(descriptor.route);
      }
    };
  }

  async negotiate(value: unknown): Promise<BridgeNegotiationReceipt> {
    const request = parseBridgeHelloRequest(value);
    return parseBridgeNegotiationReceipt(
      await this.#negotiate(request),
      request,
    );
  }

  async invoke(route: string, value: unknown): Promise<unknown> {
    const registration = this.#operations.get(route);
    if (registration === undefined) {
      throw new Error(`unknown bridge operation route: ${route}`);
    }
    const request = registration.descriptor.request.parse(value);
    return registration.descriptor.reply.parse(
      await registration.handler(request),
    );
  }
}
