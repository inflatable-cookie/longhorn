import type { BridgeHelloRequest } from "./generated/protocol.ts";
import type { BridgeOperationDescriptor } from "./operation.ts";
import { BridgeHostRouter } from "./router.ts";

export class DirectBridgeAdapter {
  readonly #router: BridgeHostRouter;

  constructor(router: BridgeHostRouter) {
    this.#router = router;
  }

  negotiate(request: BridgeHelloRequest): Promise<unknown> {
    return this.#router.negotiate(request);
  }

  invoke<Request, Reply>(
    operation: BridgeOperationDescriptor<Request, Reply>,
    request: Request,
  ): Promise<unknown> {
    return this.#router.invoke(operation.route, request);
  }
}
