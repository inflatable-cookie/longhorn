import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import type { InvokeTransport } from "@longhorn/core";

import {
  TRANSFER_COMMIT_SURFACE_COMMAND,
  TRANSFER_START_SURFACE_COMMAND,
  SurfaceTransferClient,
} from "@longhorn/surface-transfer";

const fixturePath = new URL(
  "../../../fixtures/surface-transfer/protocol-v1.json",
  import.meta.url,
);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

test("routes Surface operations through the shared transport seam", async () => {
  const transport = new MockTransport(
    new Map([
      [
        TRANSFER_START_SURFACE_COMMAND,
        array(fixture.session_responses)[0],
      ],
      [
        TRANSFER_COMMIT_SURFACE_COMMAND,
        array(fixture.commit_responses)[0],
      ],
    ]),
  );
  const client = new SurfaceTransferClient(transport);

  expect(
    await client.start(array(fixture.session_requests)[0] as never),
  ).toEqual(array(fixture.session_responses)[0] as never);
  expect(
    await client.commit(array(fixture.commit_requests)[0] as never),
  ).toEqual(array(fixture.commit_responses)[0] as never);
  expect(transport.calls.map((call) => call.command)).toEqual([
    TRANSFER_START_SURFACE_COMMAND,
    TRANSFER_COMMIT_SURFACE_COMMAND,
  ]);
  for (const call of transport.calls) {
    expect(Object.keys(call.arguments_)).toEqual(["request"]);
  }
});

class MockTransport implements InvokeTransport {
  readonly calls: Array<{
    command: string;
    arguments_: Record<string, unknown>;
  }> = [];
  readonly #responses: Map<string, unknown>;

  constructor(responses: Map<string, unknown>) {
    this.#responses = responses;
  }

  async invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown> {
    this.calls.push({ command, arguments_ });
    if (!this.#responses.has(command)) {
      throw new Error(`unexpected command ${command}`);
    }
    return this.#responses.get(command);
  }
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError("expected JSON object");
  }
  return value as Record<string, unknown>;
}

function array(value: unknown): unknown[] {
  if (!Array.isArray(value)) {
    throw new TypeError("expected JSON array");
  }
  return value;
}
