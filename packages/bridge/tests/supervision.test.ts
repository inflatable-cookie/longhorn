import { expect, test } from "bun:test";

import {
  BridgeServiceRuntime,
  BridgeSupervisionError,
  type BridgeServiceSupervisorPort,
} from "@inflatable-cookie/longhorn-bridge/supervision";

class Port implements BridgeServiceSupervisorPort {
  readonly requests: unknown[] = [];
  readonly #outcomes: unknown[];

  constructor(outcomes: unknown[]) {
    this.#outcomes = [...outcomes];
  }

  perform(request: unknown): unknown {
    this.requests.push(request);
    return this.#outcomes.shift();
  }
}

test("owned local service spawn readiness restart and shutdown are receipted", async () => {
  const runtime = new BridgeServiceRuntime("ownedLocal");
  const port = new Port([
    "accepted",
    "notReady",
    "ready",
    "accepted",
    "stopped",
  ]);

  expect(
    (
      await runtime.execute(
        port,
        "spawn",
        "credential:workspace",
      )
    ).current,
  ).toBe("starting");
  expect(
    (await runtime.execute(port, "checkReadiness")).current,
  ).toBe("awaitingReadiness");
  expect(
    (await runtime.execute(port, "checkReadiness")).current,
  ).toBe("ready");
  expect((await runtime.execute(port, "restart")).current).toBe("restarting");
  expect((await runtime.execute(port, "shutdown")).current).toBe("stopped");
  expect(port.requests[0]).toEqual({
    action: "spawn",
    credentialRef: "credential:workspace",
  });
});

test("remote attach can reconnect but cannot stop or replace its host", async () => {
  const runtime = new BridgeServiceRuntime("externalRemote");
  const port = new Port(["accepted", "ready", "accepted"]);
  await runtime.execute(port, "attach");
  await runtime.execute(port, "checkReadiness");

  for (const action of ["restart", "shutdown"] as const) {
    try {
      await runtime.execute(port, action);
      throw new Error("expected ownership rejection");
    } catch (error) {
      expect(error).toBeInstanceOf(BridgeSupervisionError);
      expect((error as BridgeSupervisionError).code).toBe(
        "lifecycle_not_owned",
      );
    }
  }
  expect((await runtime.execute(port, "reconnect")).current).toBe(
    "reconnecting",
  );
});

test("external local attach and coded failure are observable", async () => {
  const runtime = new BridgeServiceRuntime("externalLocal");
  const port = new Port([
    "accepted",
    { failed: "readinessFailed" },
  ]);
  expect((await runtime.execute(port, "attach")).current).toBe("attaching");
  const failed = await runtime.execute(port, "checkReadiness");
  expect(failed.current).toBe("failed");
  expect(failed.outcome).toEqual({ failed: "readinessFailed" });
});

test("supervision accepts no credential material or arbitrary failure text", async () => {
  const runtime = new BridgeServiceRuntime("externalLocal");
  const port = new Port([{ failed: "attachFailed", detail: "token=secret" }]);
  await expect(runtime.execute(port, "attach")).rejects.toBeInstanceOf(
    BridgeSupervisionError,
  );
  expect(JSON.stringify(port.requests)).not.toContain("secret");

  const throwing = new BridgeServiceRuntime("externalLocal");
  const unsafePort: BridgeServiceSupervisorPort = {
    perform: () => {
      throw new Error("token=secret");
    },
  };
  try {
    await throwing.execute(unsafePort, "attach");
    throw new Error("expected redacted adapter failure");
  } catch (error) {
    expect(error).toBeInstanceOf(BridgeSupervisionError);
    expect((error as Error).message).not.toContain("secret");
    expect((error as BridgeSupervisionError).code).toBe("adapter_failed");
  }

  const observing = new BridgeServiceRuntime("externalLocal");
  expect(() =>
    observing.observe("attach", {
      failed: "attachFailed",
      detail: "token=secret",
    })
  ).toThrow(BridgeSupervisionError);
  expect(() => new BridgeServiceRuntime("invalid" as any)).toThrow(
    BridgeSupervisionError,
  );
});
