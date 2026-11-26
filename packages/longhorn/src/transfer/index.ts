export * from "./generated/protocol.ts";
// The generated field map is public API. A consumer that mirrors these
// types structurally -- Poodle does, because it may not depend on Longhorn --
// has no compile-time link to them, so a field added here is silent on their
// side. Whoever sees both can assert their shape against this map and fail
// when it drifts.
export * from "./generated/fields.ts";
export * from "./validation.ts";
export * from "./client.ts";
export * from "./connection.ts";
export {
  assertTransferPayload,
  InvalidTransferPayloadError,
  LONGHORN_TRANSFER_MIME_TYPE,
  parseTransferPayload,
  serializeTransferPayload,
} from "./payload.ts";
