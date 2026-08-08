import { describe, expect, test } from "bun:test";

import {
  createDirectOperationPort,
  OperationClient,
  OperationResponseCorrelationError,
  SerializedOperationPort,
} from "../../src/operation/index.ts";
import { fixture, fixturePort } from "./support.ts";

describe("operation transport conformance", () => {
  test("direct and serialized traces converge", async () => {
    for (const port of [
      createDirectOperationPort(fixturePort()),
      new SerializedOperationPort(fixturePort()),
    ]) {
      const client = new OperationClient(port);
      expect(await client.snapshot()).toEqual(fixture.snapshotResponse);
      for (const [index, command] of fixture.mutationCommands.entries()) {
        expect(await client.mutate(command)).toEqual(fixture.mutationResults[index]);
      }
      expect(await client.cancel(fixture.cancellationCommand)).toEqual(fixture.cancellationResult);
    }
  });

  test("rejects transport replies carrying another operation request id", async () => {
    const port = fixturePort();
    const client = new OperationClient({
      ...port,
      snapshot: async () => ({ ...fixture.snapshotResponse, requestId: "request:foreign" }),
    });
    expect(client.snapshot()).rejects.toBeInstanceOf(OperationResponseCorrelationError);
  });
});
