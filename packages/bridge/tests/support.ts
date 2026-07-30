import { readFileSync } from "node:fs";

import {
  bridgeCodec,
  record,
  type BridgeCodec,
} from "@longhorn/bridge";

export const fixture = record(
  JSON.parse(
    readFileSync(
      new URL("../../../fixtures/bridge/protocol-v1.json", import.meta.url),
      "utf8",
    ),
  ),
  [
    "protocolVersion",
    "hello",
    "negotiation",
    "queryRequests",
    "queryReplies",
    "commandRequests",
    "commandReplies",
    "snapshot",
    "events",
    "progress",
    "cancellationRequest",
    "cancellationReceipt",
    "terminal",
    "semanticTrace",
    "incompatibility",
  ],
);

export interface QueryPayload {
  readonly includeArchived: boolean;
}

export interface CommandPayload {
  readonly delta: number;
}

export interface SuccessPayload {
  readonly value: number;
}

export interface FailureDetail {
  readonly source: string;
}

export const queryPayloadCodec: BridgeCodec<QueryPayload> = bridgeCodec(
  (value) => {
    const source = record(value, ["includeArchived"]);
    if (typeof source.includeArchived !== "boolean") {
      throw new TypeError("includeArchived must be boolean");
    }
    return { includeArchived: source.includeArchived };
  },
);

export const commandPayloadCodec: BridgeCodec<CommandPayload> = bridgeCodec(
  (value) => {
    const source = record(value, ["delta"]);
    if (typeof source.delta !== "number" || !Number.isSafeInteger(source.delta)) {
      throw new TypeError("delta must be an integer");
    }
    return { delta: source.delta };
  },
);

export const successPayloadCodec: BridgeCodec<SuccessPayload> = bridgeCodec(
  (value) => {
    const source = record(value, ["value"]);
    if (typeof source.value !== "number" || !Number.isSafeInteger(source.value)) {
      throw new TypeError("value must be an integer");
    }
    return { value: source.value };
  },
);

export const failureDetailCodec: BridgeCodec<FailureDetail> = bridgeCodec(
  (value) => {
    const source = record(value, ["source"]);
    if (typeof source.source !== "string") {
      throw new TypeError("source must be a string");
    }
    return { source: source.source };
  },
);

export const jsonCodec = bridgeCodec<unknown>((value) => value);

export function values(value: unknown): readonly unknown[] {
  if (!Array.isArray(value)) {
    throw new TypeError("fixture category must be an array");
  }
  return value;
}
