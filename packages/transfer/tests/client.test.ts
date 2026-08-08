import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import type { InvokeTransport } from "@inflatable-cookie/longhorn-core";

import {
  TRANSFER_CANCEL_COMMAND,
  TRANSFER_COMMIT_PANEL_COMMAND,
  TRANSFER_PUBLISH_LEASE_COMMAND,
  TRANSFER_SNAPSHOT_COMMAND,
  TRANSFER_START_PANEL_COMMAND,
  TransferClient,
} from "@inflatable-cookie/longhorn-transfer";

const fixturePath = new URL(
  "../../../fixtures/transfer/protocol-v1.json",
  import.meta.url,
);
const fixture = record(JSON.parse(readFileSync(fixturePath, "utf8")));

test("routes each typed operation through the injected transport", async () => {
  const responses = new Map<string, unknown>([
    [TRANSFER_SNAPSHOT_COMMAND, fixture.client_snapshot],
    [
      TRANSFER_START_PANEL_COMMAND,
      array(fixture.session_responses)[0],
    ],
    [
      TRANSFER_PUBLISH_LEASE_COMMAND,
      array(fixture.lease_responses)[0],
    ],
    [
      TRANSFER_COMMIT_PANEL_COMMAND,
      array(fixture.commit_responses)[0],
    ],
    [TRANSFER_CANCEL_COMMAND, array(fixture.cancel_responses)[0]],
  ]);
  const transport = new MockTransport(responses);
  const client = new TransferClient(transport);

  expect(await client.snapshot()).toEqual(fixture.client_snapshot as never);
  expect(
    await client.startPanel(array(fixture.session_requests)[0] as never),
  ).toEqual(array(fixture.session_responses)[0] as never);
  expect(
    await client.publishLease(array(fixture.lease_requests)[0] as never),
  ).toEqual(array(fixture.lease_responses)[0] as never);
  expect(
    await client.commitPanel(array(fixture.commit_requests)[0] as never),
  ).toEqual(array(fixture.commit_responses)[0] as never);
  expect(
    await client.cancel(array(fixture.cancel_requests)[0] as never),
  ).toEqual(array(fixture.cancel_responses)[0] as never);

  expect(transport.calls.map((call) => call.command)).toEqual([
    TRANSFER_SNAPSHOT_COMMAND,
    TRANSFER_START_PANEL_COMMAND,
    TRANSFER_PUBLISH_LEASE_COMMAND,
    TRANSFER_COMMIT_PANEL_COMMAND,
    TRANSFER_CANCEL_COMMAND,
  ]);
  expect(transport.calls[0]?.arguments_).toEqual({});
  for (const call of transport.calls.slice(1)) {
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
