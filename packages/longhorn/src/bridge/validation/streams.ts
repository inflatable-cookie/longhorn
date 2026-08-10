import { BRIDGE_FIELDS } from "../generated/fields.ts";
import type {
  BridgeEventEnvelope,
  BridgeSnapshotEnvelope,
  BridgeStreamCursor,
} from "../generated/protocol.ts";
import {
  type BridgeCodec,
  domainId,
  integer,
  opaqueId,
  record,
} from "./base.ts";

export function parseBridgeStreamCursor(
  value: unknown,
): BridgeStreamCursor {
  const source = record(value, BRIDGE_FIELDS.BridgeStreamCursor);
  return {
    sessionId: opaqueId(source.sessionId),
    domainId: domainId(source.domainId),
    authorityEpoch: integer(source.authorityEpoch, 1),
    sequence: integer(source.sequence),
  };
}

export function parseBridgeSnapshotEnvelope<P>(
  value: unknown,
  payload: BridgeCodec<P>,
): BridgeSnapshotEnvelope<P> {
  const source = record(value, BRIDGE_FIELDS.BridgeEventEnvelope);
  return {
    cursor: parseBridgeStreamCursor(source.cursor),
    payload: payload.parse(source.payload),
  };
}

export function parseBridgeEventEnvelope<P>(
  value: unknown,
  payload: BridgeCodec<P>,
): BridgeEventEnvelope<P> {
  const source = record(value, BRIDGE_FIELDS.BridgeEventEnvelope);
  return {
    cursor: parseBridgeStreamCursor(source.cursor),
    payload: payload.parse(source.payload),
  };
}
