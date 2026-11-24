import { describe, expect, test } from "bun:test";

import { CommandClient } from "../../src/commands/client.ts";
import type {
  CommandAvailabilityPort,
  CommandCataloguePort,
  CommandKeymapPort,
  CommandPorts,
  CommandUnlisten,
} from "../../src/commands/ports.ts";

interface ListenStub {
  listen?: (invalidate: () => void) => CommandUnlisten | Promise<CommandUnlisten>;
}

function stubPort<Port extends { listen?: unknown }>(stub: ListenStub): Port {
  return {
    load: async () => {
      throw new Error("not loaded in listen tests");
    },
    ...stub,
  } as unknown as Port;
}

function clientWith(stubs: {
  catalogue: ListenStub;
  keymap: ListenStub;
  availability: ListenStub;
}): CommandClient {
  const ports: CommandPorts = {
    catalogue: stubPort<CommandCataloguePort>(stubs.catalogue),
    keymap: stubPort<CommandKeymapPort>(stubs.keymap),
    availability: stubPort<CommandAvailabilityPort>(stubs.availability),
    executor: { execute: async () => ({ status: "succeeded" }) },
    nextRequestId: () => "request:listen",
  };
  return new CommandClient(ports);
}

describe("CommandClient listen", () => {
  test("a synchronous registration throw disposes every earlier registration", async () => {
    const boom = new Error("keymap registration refused");
    let disposed = 0;
    const client = clientWith({
      catalogue: { listen: () => () => void (disposed += 1) },
      keymap: {
        listen: () => {
          throw boom;
        },
      },
      availability: { listen: () => () => void (disposed += 1) },
    });

    await expect(client.listen(() => {})).rejects.toBe(boom);
    expect(disposed).toBe(2);
  });

  test("a cleanup that throws synchronously does not replace the registration error", async () => {
    const boom = new Error("keymap registration refused");
    let disposed = 0;
    const client = clientWith({
      catalogue: {
        listen: () => () => {
          disposed += 1;
          throw new Error("cleanup refused");
        },
      },
      keymap: {
        listen: () => {
          throw boom;
        },
      },
      availability: { listen: () => () => void (disposed += 1) },
    });

    await expect(client.listen(() => {})).rejects.toBe(boom);
    expect(disposed).toBe(2);
  });

  test("returns every unlisten in port order when all registrations succeed", async () => {
    const order: Array<string> = [];
    const client = clientWith({
      catalogue: { listen: () => () => void order.push("catalogue") },
      keymap: { listen: () => () => void order.push("keymap") },
      availability: { listen: () => () => void order.push("availability") },
    });

    const unlisten = await client.listen(() => {});
    expect(unlisten).toHaveLength(3);
    for (const dispose of unlisten) {
      await dispose();
    }
    expect(order).toEqual(["catalogue", "keymap", "availability"]);
  });
});
