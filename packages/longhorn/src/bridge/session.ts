import type {
  BridgeHelloRequest,
  BridgeNegotiationReceipt,
  DomainAuthorityDescriptor,
  DomainCapabilityDescriptor,
  DomainId,
} from "./generated/protocol.ts";
import {
  parseBridgeHelloRequest,
  parseBridgeNegotiationReceipt,
} from "./compatibility.ts";

export interface BridgeNegotiationAdapter {
  negotiate(request: BridgeHelloRequest): Promise<unknown>;
}

export class BridgeSessionClient {
  readonly #adapter: BridgeNegotiationAdapter;

  constructor(adapter: BridgeNegotiationAdapter) {
    this.#adapter = adapter;
  }

  async connect(request: BridgeHelloRequest): Promise<BridgeSession> {
    const checkedRequest = parseBridgeHelloRequest(request);
    const receipt = parseBridgeNegotiationReceipt(
      await this.#adapter.negotiate(checkedRequest),
      checkedRequest,
    );
    return new BridgeSession(receipt);
  }
}

export class BridgeSession {
  readonly receipt: BridgeNegotiationReceipt;
  readonly #capabilities: ReadonlyMap<DomainId, DomainCapabilityDescriptor>;
  readonly #authorities: ReadonlyMap<DomainId, DomainAuthorityDescriptor>;

  constructor(receipt: BridgeNegotiationReceipt) {
    this.receipt = parseBridgeNegotiationReceipt(receipt);
    this.#capabilities = new Map(
      this.receipt.domainCapabilities.map((value) => [
        value.domainId,
        value,
      ]),
    );
    this.#authorities = new Map(
      this.receipt.domainAuthorities.map((value) => [
        value.domainId,
        value,
      ]),
    );
  }

  capability(domainId: DomainId): DomainCapabilityDescriptor | undefined {
    return this.#capabilities.get(domainId);
  }

  authority(domainId: DomainId): DomainAuthorityDescriptor | undefined {
    return this.#authorities.get(domainId);
  }

  supports(domainId: DomainId, capability: string): boolean {
    return this.capability(domainId)?.capabilities.includes(capability) ??
      false;
  }
}
