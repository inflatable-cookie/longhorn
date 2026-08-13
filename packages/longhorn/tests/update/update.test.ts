import { describe, expect, test } from "bun:test";
import {
  UPDATE_PROTOCOL_VERSION,
  UpdateClient,
  UpdateController,
  UpdateValidationError,
  assertUpdateOutcome,
  assertUpdateSnapshot,
  createDirectUpdatePort,
  SerializedUpdatePort,
  type Channel,
  type DeferralCause,
  type UpdateAvailabilityProjection,
  type UpdateChangedEvent,
  type UpdateOutcomeProjection,
  type UpdatePort,
  type UpdateProgressProjection,
  type UpdateSnapshot,
} from "../../src/update/index.ts";

function snapshot(overrides: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return {
    protocolVersion: UPDATE_PROTOCOL_VERSION,
    authorityEpoch: 3,
    channel: "production",
    installedVersion: "1.3.0",
    availability: { state: "upToDate" },
    deferral: null,
    progress: { state: "idle" },
    ...overrides,
  };
}

function committed(value: UpdateSnapshot = snapshot()): UpdateOutcomeProjection {
  return { status: "committed", snapshot: value };
}

class Port implements UpdatePort {
  calls: string[] = [];
  constructor(
    private state: UpdateSnapshot = snapshot(),
    private outcome: (command: string) => UpdateOutcomeProjection = () => committed(this.state),
  ) {}
  async snapshot(): Promise<unknown> { this.calls.push("snapshot"); return this.state; }
  async check(): Promise<unknown> { this.calls.push("check"); return this.outcome("check"); }
  async selectChannel(): Promise<unknown> { this.calls.push("selectChannel"); return this.outcome("selectChannel"); }
  async defer(): Promise<unknown> { this.calls.push("defer"); return this.outcome("defer"); }
  async install(): Promise<unknown> { this.calls.push("install"); return this.outcome("install"); }
  async listen(): Promise<() => void> { this.calls.push("listen"); return () => {}; }
}

describe("update validation", () => {
  test("a well-formed snapshot passes", () => {
    expect(() => assertUpdateSnapshot(snapshot())).not.toThrow();
  });

  test("an unknown key is rejected", () => {
    expect(() => assertUpdateSnapshot({ ...snapshot(), extra: 1 })).toThrow(UpdateValidationError);
  });

  test("a surplus key on a union variant is rejected", () => {
    const availability = { state: "upToDate", version: "1.4.0" } as unknown as UpdateAvailabilityProjection;
    expect(() => assertUpdateSnapshot(snapshot({ availability }))).toThrow(UpdateValidationError);
  });

  test("a wrong protocol line is rejected", () => {
    expect(() => assertUpdateSnapshot({ ...snapshot(), protocolVersion: 2 })).toThrow(
      UpdateValidationError,
    );
  });

  /**
   * The absent fraction is the protocol's own answer for a source that
   * declares no content length. A validator that rejected it, or coerced it to
   * zero, would put back the invented number the protocol avoided.
   */
  test("a downloading state with no fraction passes, and zero is not substituted", () => {
    const progress: UpdateProgressProjection = { state: "downloading", fraction: null };
    expect(() => assertUpdateSnapshot(snapshot({ progress }))).not.toThrow();
  });

  test("a fraction outside zero to one is rejected", () => {
    const progress = { state: "downloading", fraction: 1.5 } as unknown as UpdateProgressProjection;
    expect(() => assertUpdateSnapshot(snapshot({ progress }))).toThrow(UpdateValidationError);
  });

  test("a rejected outcome carries a known code and the state as it remains", () => {
    const outcome: UpdateOutcomeProjection = {
      status: "rejected",
      code: "notWritable",
      snapshot: snapshot(),
    };
    expect(() => assertUpdateOutcome(outcome)).not.toThrow();
    expect(() => assertUpdateOutcome({ ...outcome, code: "invented" })).toThrow(
      UpdateValidationError,
    );
  });

  test("a product payload field is refused anywhere in the tree", () => {
    expect(() =>
      assertUpdateSnapshot({ ...snapshot(), progress: { state: "idle", payload: {} } }),
    ).toThrow(UpdateValidationError);
  });
});

describe("update client", () => {
  test("an outbound command is validated before it is sent", async () => {
    const port = new Port();
    const client = new UpdateClient(port);

    await expect(
      client.install({
        protocolVersion: UPDATE_PROTOCOL_VERSION,
        authorityEpoch: 3,
        version: "",
      }),
    ).rejects.toThrow(UpdateValidationError);
    expect(port.calls).toEqual([]);
  });

  test("a malformed reply is refused rather than returned", async () => {
    const port = createDirectUpdatePort({
      snapshot: async () => ({ nonsense: true }),
      check: async () => committed(),
      selectChannel: async () => committed(),
      defer: async () => committed(),
      install: async () => committed(),
    });

    await expect(new UpdateClient(port).snapshot()).rejects.toThrow(UpdateValidationError);
  });

  test("a listened event is validated before the listener sees it", async () => {
    let delivered: UpdateChangedEvent | undefined;
    let leaked = false;
    const port = createDirectUpdatePort({
      snapshot: async () => snapshot(),
      check: async () => committed(),
      selectChannel: async () => committed(),
      defer: async () => committed(),
      install: async () => committed(),
      listen: (listener) => {
        expect(() => listener({ kind: "not a kind" })).toThrow(UpdateValidationError);
        leaked = true;
        listener({
          protocolVersion: UPDATE_PROTOCOL_VERSION,
          authorityEpoch: 3,
          kind: "checked",
        });
        return () => {};
      },
    });

    await new UpdateClient(port).listen((event) => {
      delivered = event;
    });

    expect(leaked).toBeTrue();
    expect(delivered?.kind).toBe("checked");
  });

  test("the serialized port survives a structured-clone round trip", async () => {
    const port = new SerializedUpdatePort(new Port());

    await expect(new UpdateClient(port).snapshot()).resolves.toEqual(snapshot());
  });
});

describe("update controller", () => {
  test("start reads the snapshot and reports ready", async () => {
    const controller = new UpdateController({ port: new Port() });

    await controller.start();

    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.installedVersion).toBe("1.3.0");
    expect(controller.channel).toBe("production" satisfies Channel);
  });

  /**
   * Card 154 step 5. An install ahead of its channel receives nothing until
   * the channel catches up, which is correct and reads as a broken updater
   * unless the surface can say so.
   */
  test("ahead-of-channel is readable on its own, not folded into up-to-date", async () => {
    const ahead = snapshot({
      availability: { state: "aheadOfChannel", installed: "1.3.0-nightly.4", channel: "1.2.9" },
    });
    const controller = new UpdateController({ port: new Port(ahead) });

    await controller.start();

    expect(controller.aheadOfChannel).toEqual({
      installed: "1.3.0-nightly.4",
      channel: "1.2.9",
    });
    expect(controller.availability?.state).not.toBe("upToDate");
  });

  test("up-to-date reports no ahead-of-channel", async () => {
    const controller = new UpdateController({ port: new Port() });

    await controller.start();

    expect(controller.aheadOfChannel).toBeUndefined();
  });

  /**
   * Card 154 step 6. The gate refusing an install is a committed outcome with
   * a reason, not a failure. A surface that reported it as one would tell a
   * customer their update is broken when nothing is.
   */
  test("a gated install stays ready and shows its reason", async () => {
    const cause: DeferralCause = { cause: "workInFlight", detail: "1 open transfer session" };
    const gated = snapshot({ deferral: { version: "1.4.0", cause } });
    const controller = new UpdateController({
      port: new Port(snapshot(), () => committed(gated)),
    });

    await controller.start();
    await controller.install("1.4.0");

    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.deferral).toEqual({ version: "1.4.0", cause });
    expect(controller.lastRejection).toBeUndefined();
    expect(controller.installedVersion).toBe("1.3.0");
  });

  test("a rejection is distinct from a deferral and carries its code", async () => {
    const controller = new UpdateController({
      port: new Port(snapshot(), () => ({
        status: "rejected",
        code: "notWritable",
        snapshot: snapshot(),
      })),
    });

    await controller.start();
    await controller.install("1.4.0");

    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.lastRejection).toBe("notWritable");
    expect(controller.deferral).toBeUndefined();
  });

  test("a later committed command clears the previous rejection", async () => {
    let refuse = true;
    const controller = new UpdateController({
      port: new Port(snapshot(), () =>
        refuse ? { status: "rejected", code: "unreachable", snapshot: snapshot() } : committed(),
      ),
    });

    await controller.start();
    await controller.install("1.4.0");
    expect(controller.lastRejection).toBe("unreachable");

    refuse = false;
    await controller.check();

    expect(controller.lastRejection).toBeUndefined();
  });

  test("a command sends the epoch the last snapshot carried", async () => {
    const port = new Port(snapshot({ authorityEpoch: 9 }));
    const client = new UpdateClient(port);
    const sent: number[] = [];
    const controller = new UpdateController({
      port: {
        snapshot: () => client.snapshot(),
        check: async (command) => {
          sent.push(command.authorityEpoch);
          return committed(snapshot({ authorityEpoch: 9 }));
        },
        selectChannel: async () => committed(),
        defer: async () => committed(),
        install: async () => committed(),
      },
    });

    await controller.start();
    await controller.check();

    expect(sent).toEqual([9]);
  });

  /** Inventing an epoch would have the authority refuse it as stale, which
   * reads as a protocol problem rather than as "nothing has been read yet". */
  test("a command before the first read fails saying so", async () => {
    const controller = new UpdateController({ port: new Port() });

    await controller.check();

    expect(controller.status.kind).toBe("failed");
    expect(String((controller.status as { error: unknown }).error)).toContain("has not been read");
  });

  test("stop releases the listener and returns to idle", async () => {
    let released = false;
    const controller = new UpdateController({
      port: createDirectUpdatePort({
        snapshot: async () => snapshot(),
        check: async () => committed(),
        selectChannel: async () => committed(),
        defer: async () => committed(),
        install: async () => committed(),
        listen: () => () => {
          released = true;
        },
      }),
    });

    await controller.start();
    await controller.stop();

    expect(released).toBeTrue();
    expect(controller.status).toEqual({ kind: "idle" });
  });

  test("observers see each state change", async () => {
    const controller = new UpdateController({ port: new Port() });
    let notifications = 0;
    controller.observe(() => {
      notifications += 1;
    });

    await controller.start();

    expect(notifications).toBeGreaterThan(0);
  });
});
