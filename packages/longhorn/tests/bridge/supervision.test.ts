import { expect, test } from "bun:test";

import {
  BridgeServiceRuntime,
  BridgeSupervisionError,
} from "@inflatable-cookie/longhorn/bridge/supervision";
import type { BridgeServiceSupervisorPort } from "@inflatable-cookie/longhorn/bridge/supervision";

class Port implements BridgeServiceSupervisorPort {
  readonly requests: Array<unknown> = [];
  readonly #outcomes: Array<unknown>;

  constructor(outcomes: Array<unknown>) {
    this.#outcomes = [...outcomes];
  }

  perform(request: unknown): unknown {
    this.requests.push(request);
    return this.#outcomes.shift();
  }
}

test("owned local service spawn readiness restart and shutdown are receipted", async () => {
  const runtime = new BridgeServiceRuntime("ownedLocal");
  const port = new Port(["accepted", "notReady", "ready", "accepted", "stopped"]);

  expect((await runtime.execute(port, "spawn", "credential:workspace")).current).toBe("starting");
  expect((await runtime.execute(port, "checkReadiness")).current).toBe("awaitingReadiness");
  expect((await runtime.execute(port, "checkReadiness")).current).toBe("ready");
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
      expect((error as BridgeSupervisionError).code).toBe("lifecycle_not_owned");
    }
  }
  expect((await runtime.execute(port, "reconnect")).current).toBe("reconnecting");
});

test("external local attach and coded failure are observable", async () => {
  const runtime = new BridgeServiceRuntime("externalLocal");
  const port = new Port(["accepted", { failed: "readinessFailed" }]);
  expect((await runtime.execute(port, "attach")).current).toBe("attaching");
  const failed = await runtime.execute(port, "checkReadiness");
  expect(failed.current).toBe("failed");
  expect(failed.outcome).toEqual({ failed: "readinessFailed" });
});

test("a delayed readiness probe cannot report ready for a superseded service", async () => {
  const runtime = new BridgeServiceRuntime("ownedLocal");
  const port = new Port(["accepted", "ready"]);
  await runtime.execute(port, "spawn");
  await runtime.execute(port, "checkReadiness");
  expect(runtime.state).toBe("ready");

  // Admit the probe against the ready service, restart before it reports.
  const probe = runtime.begin("checkReadiness");
  expect(runtime.observe("restart", "accepted").current).toBe("restarting");
  const afterRestart = runtime.begin("checkReadiness");
  expect(afterRestart.generation).toBeGreaterThan(probe.generation);

  expect(() => runtime.commit(probe, "ready")).toThrow(BridgeSupervisionError);
  expect(runtime.state).toBe("restarting");

  // A probe admitted after the restart still reports readiness.
  expect(runtime.commit(afterRestart, "ready").current).toBe("ready");
});

test("a readiness probe awaiting its adapter cannot report ready after a restart", async () => {
  const runtime = new BridgeServiceRuntime("ownedLocal");
  runtime.observe("spawn", "accepted");
  runtime.observe("checkReadiness", "ready");

  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const probe: BridgeServiceSupervisorPort = {
    perform: async () => {
      await gate;
      return "ready";
    },
  };

  const pending = runtime.execute(probe, "checkReadiness");
  runtime.observe("restart", "accepted");
  release();

  await expect(pending).rejects.toBeInstanceOf(BridgeSupervisionError);
  expect(runtime.state).toBe("restarting");
});

test("admission checks the same ownership and state rules as execution", () => {
  const owned = new BridgeServiceRuntime("ownedLocal");
  expect(() => owned.begin("shutdown")).toThrow(BridgeSupervisionError);

  const external = new BridgeServiceRuntime("externalRemote");
  try {
    external.begin("restart");
    throw new Error("expected ownership rejection");
  } catch (error) {
    expect((error as BridgeSupervisionError).code).toBe("lifecycle_not_owned");
  }
});

test("supervision accepts no credential material or arbitrary failure text", async () => {
  const runtime = new BridgeServiceRuntime("externalLocal");
  const port = new Port([{ failed: "attachFailed", detail: "token=secret" }]);
  await expect(runtime.execute(port, "attach")).rejects.toBeInstanceOf(BridgeSupervisionError);
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
    }),
  ).toThrow(BridgeSupervisionError);
  expect(() => new BridgeServiceRuntime("invalid" as any)).toThrow(BridgeSupervisionError);
});
