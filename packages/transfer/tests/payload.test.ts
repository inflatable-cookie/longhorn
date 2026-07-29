import { describe, expect, test } from "bun:test";

import {
  InvalidTransferPayloadError,
  parseTransferPayload,
  serializeTransferPayload,
} from "@longhorn/transfer";

describe("native transfer payload", () => {
  test("contains only protocol version and host-issued session identity", () => {
    const serialized = serializeTransferPayload({
      protocol_version: 1,
      session_id: "abababababababababababababababab",
    });

    expect(JSON.parse(serialized)).toEqual({
      protocol_version: 1,
      session_id: "abababababababababababababababab",
    });
    expect(parseTransferPayload(serialized)).toEqual({
      protocol_version: 1,
      session_id: "abababababababababababababababab",
    });
    expect(serialized).not.toMatch(
      /panel|surface|layout|window|document|binding/,
    );
  });

  test("rejects extra authority, future versions, and invented ids", () => {
    for (const payload of [
      {
        protocol_version: 1,
        session_id: "abababababababababababababababab",
        panel_instance_id: "instance:a",
      },
      {
        protocol_version: 2,
        session_id: "abababababababababababababababab",
      },
      { protocol_version: 1, session_id: "renderer-fallback" },
    ]) {
      expect(() => parseTransferPayload(JSON.stringify(payload))).toThrow(
        InvalidTransferPayloadError,
      );
    }
  });
});
