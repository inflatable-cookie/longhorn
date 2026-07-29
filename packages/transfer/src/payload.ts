import {
  TRANSFER_PROTOCOL_VERSION,
  type TransferPayload,
} from "./generated/protocol.ts";

export const LONGHORN_TRANSFER_MIME_TYPE =
  "application/vnd.longhorn.transfer+json";

export class InvalidTransferPayloadError extends TypeError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidTransferPayloadError";
  }
}

export function serializeTransferPayload(payload: TransferPayload): string {
  assertTransferPayload(payload);
  return JSON.stringify({
    protocol_version: payload.protocol_version,
    session_id: payload.session_id,
  });
}

export function parseTransferPayload(value: string): TransferPayload {
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new InvalidTransferPayloadError("transfer payload is not valid JSON");
  }
  assertTransferPayload(parsed);
  return parsed;
}

export function assertTransferPayload(
  value: unknown,
): asserts value is TransferPayload {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value)
  ) {
    throw new InvalidTransferPayloadError(
      "transfer payload must be an object",
    );
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record).sort();
  if (
    keys.length !== 2 ||
    keys[0] !== "protocol_version" ||
    keys[1] !== "session_id"
  ) {
    throw new InvalidTransferPayloadError(
      "transfer payload must contain only protocol_version and session_id",
    );
  }
  if (record.protocol_version !== TRANSFER_PROTOCOL_VERSION) {
    throw new InvalidTransferPayloadError(
      "transfer payload protocol version is unsupported",
    );
  }
  if (
    typeof record.session_id !== "string" ||
    !/^[0-9a-f]{32}$/.test(record.session_id)
  ) {
    throw new InvalidTransferPayloadError(
      "transfer session id must be 32 lowercase hexadecimal characters",
    );
  }
}
