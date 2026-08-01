import { describe, expect, test } from "bun:test";

import { NativeContentClient } from "../src/index.ts";
import {
  NATIVE_CONTENT_CHANGED_EVENT,
  NATIVE_CONTENT_CONNECT_COMMAND,
  NATIVE_CONTENT_DECIDE_SIZE_COMMAND,
  NATIVE_CONTENT_SNAPSHOT_COMMAND,
  NATIVE_CONTENT_UPDATE_DESIRED_COMMAND,
  createTauriNativeContentPort,
} from "../src/tauri.ts";
import { MemoryNativeContentHost, flush, nextUpdate } from "./support.ts";

describe("Tauri-shaped transport assembly", () => {
  test("uses four narrow commands and one product-neutral event", async () => {
    const host = new MemoryNativeContentHost();
    const calls: Array<[string, unknown]> = [];
    let listenedEvent = "";
    const transport = {
      async invoke(command: string, args: Record<string, unknown>) {
        calls.push([command, args]);
        const request = args.request as never;
        switch (command) {
          case NATIVE_CONTENT_CONNECT_COMMAND:
            return host.connect(request);
          case NATIVE_CONTENT_SNAPSHOT_COMMAND:
            return host.snapshot(request);
          case NATIVE_CONTENT_UPDATE_DESIRED_COMMAND:
            return host.updateDesired(request);
          case NATIVE_CONTENT_DECIDE_SIZE_COMMAND:
            return host.decideContentSize(request);
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      async listen(event: string, listener: (payload: unknown) => void) {
        listenedEvent = event;
        return host.listen(listener);
      },
    };
    let sequence = 0;
    const port = createTauriNativeContentPort({
      transport,
      nextRequestId: () => `request:tauri-${++sequence}`,
    });
    const connection = new NativeContentClient(
      port,
      "island:fixture",
    ).connect();
    const initial = await connection.ready;
    await connection.updateDesired(nextUpdate(initial, 44));
    host.admitObservation();
    await flush();
    const current = connection.current();
    if (current === undefined) throw new Error("missing current snapshot");
    await connection.decideContentSize(
      {
        generation: current.cursor.attach_generation,
        desired_revision: current.cursor.desired_revision,
        size: { width: 800, height: 600 },
      },
      { kind: "accepted" },
    );
    await connection.dispose();

    expect(listenedEvent).toBe(NATIVE_CONTENT_CHANGED_EVENT);
    expect(new Set(calls.map(([command]) => command))).toEqual(
      new Set([
        NATIVE_CONTENT_CONNECT_COMMAND,
        NATIVE_CONTENT_SNAPSHOT_COMMAND,
        NATIVE_CONTENT_UPDATE_DESIRED_COMMAND,
        NATIVE_CONTENT_DECIDE_SIZE_COMMAND,
      ]),
    );
    for (const [, args] of calls) {
      expect(Object.keys(args as Record<string, unknown>)).toEqual(["request"]);
    }
    expect(connection.current()?.cursor.observed_revision).toBe(1);
  });
});
