import { describe, expect, test } from "bun:test";
import { ok } from "node:assert/strict";
import {
  UPDATE_PROTOCOL_VERSION,
  UpdateClient,
  UpdateController,
  UpdateControllerUnavailableError,
  UpdateNotStagedError,
  UpdateValidationError,
  assertUpdateOutcome,
  assertUpdateSnapshot,
  createDirectUpdatePort,
  SerializedUpdatePort,
} from "../../src/update/index.ts";
import type {
  Channel,
  DeferralCause,
  UpdateApplyCommand,
  UpdateAvailabilityProjection,
  UpdateOutcomeProjection,
  UpdatePort,
  UpdatePrepareCommand,
  UpdateProgressProjection,
  UpdateSnapshot,
} from "../../src/update/index.ts";

function snapshot(overrides: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return {
    protocolVersion: UPDATE_PROTOCOL_VERSION,
    authorityEpoch: 3,
    channel: "production",
    installedVersion: "1.3.0",
    availability: { state: "upToDate" },
    deferral: null,
    staged: null,
    progress: { state: "idle" },
    ...overrides,
  };
}

const STAGED = {
  version: "1.4.0",
  channel: "production" as Channel,
  digest: "ab".repeat(32),
};

function committed(value: UpdateSnapshot = snapshot()): UpdateOutcomeProjection {
  return { status: "committed", snapshot: value };
}

class Port implements UpdatePort {
  calls: Array<string> = [];
  applied: Array<UpdateApplyCommand> = [];
  constructor(
    private state: UpdateSnapshot = snapshot(),
    private outcome: (command: string) => UpdateOutcomeProjection = () => committed(this.state),
  ) {}
  async snapshot(): Promise<unknown> {
    this.calls.push("snapshot");
    return this.state;
  }
  async check(): Promise<unknown> {
    this.calls.push("check");
    return this.outcome("check");
  }
  async selectChannel(): Promise<unknown> {
    this.calls.push("selectChannel");
    return this.outcome("selectChannel");
  }
  async defer(): Promise<unknown> {
    this.calls.push("defer");
    return this.outcome("defer");
  }
  async prepare(): Promise<unknown> {
    this.calls.push("prepare");
    return this.outcome("prepare");
  }
  async apply(command: UpdateApplyCommand): Promise<unknown> {
    this.calls.push("apply");
    this.applied.push(command);
    return this.outcome("apply");
  }
  async cancel(): Promise<unknown> {
    this.calls.push("cancel");
    return this.outcome("cancel");
  }
  async listen(_listener: (event: unknown) => void): Promise<() => void> {
    this.calls.push("listen");
    return () => {};
  }
}

class PreparingPort extends Port {
  readonly #transfers: Array<{
    readonly report: (event: unknown) => void;
    readonly complete: (outcome: UpdateOutcomeProjection) => void;
  }> = [];

  prepareWithProgress(
    _command: UpdatePrepareCommand,
    report: (event: unknown) => void,
  ): Promise<unknown> {
    return new Promise<UpdateOutcomeProjection>((complete) => {
      this.#transfers.push({ report, complete });
    });
  }

  transfer(index: number) {
    const transfer = this.#transfers[index];
    ok(transfer, `expected prepare ${index + 1} to be pending`);
    return transfer;
  }
}

const CHANGED_EVENT = {
  protocolVersion: UPDATE_PROTOCOL_VERSION,
  authorityEpoch: 3,
  kind: "checked",
};
const LIVE_PROGRESS = { state: "downloading", received: 5, expected: 10, fraction: 0.5 } as const;
const PROGRESS_EVENT = {
  protocolVersion: UPDATE_PROTOCOL_VERSION,
  authorityEpoch: 3,
  progress: LIVE_PROGRESS,
};

describe("update validation", () => {
  test("a well-formed snapshot passes", () => {
    expect(() => assertUpdateSnapshot(snapshot())).not.toThrow();
  });

  test("an unknown key is rejected", () => {
    expect(() => assertUpdateSnapshot({ ...snapshot(), extra: 1 })).toThrow(UpdateValidationError);
  });

  test("a surplus key on a union variant is rejected", () => {
    const availability = {
      state: "upToDate",
      version: "1.4.0",
    } as unknown as UpdateAvailabilityProjection;
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
    const progress: UpdateProgressProjection = {
      state: "downloading",
      received: 4096,
      expected: null,
      fraction: null,
    };
    expect(() => assertUpdateSnapshot(snapshot({ progress }))).not.toThrow();
  });

  test("a fraction outside zero to one is rejected", () => {
    const progress = {
      state: "downloading",
      received: 1,
      expected: 2,
      fraction: 1.5,
    } as unknown as UpdateProgressProjection;
    expect(() => assertUpdateSnapshot(snapshot({ progress }))).toThrow(UpdateValidationError);
  });

  test("a negative byte count is rejected", () => {
    const progress = {
      state: "downloading",
      received: -1,
      expected: null,
      fraction: null,
    } as unknown as UpdateProgressProjection;
    expect(() => assertUpdateSnapshot(snapshot({ progress }))).toThrow(UpdateValidationError);
  });

  test("a staged identity is validated as its own shape", () => {
    expect(() => assertUpdateSnapshot(snapshot({ staged: STAGED }))).not.toThrow();
    const bad = { ...STAGED, digest: "not-a-digest" };
    expect(() => assertUpdateSnapshot(snapshot({ staged: bad }))).toThrow(UpdateValidationError);
  });

  test("a rejected outcome carries a known code and the state as it remains", () => {
    const outcome: UpdateOutcomeProjection = {
      status: "rejected",
      code: "notWritable",
      detail: null,
      snapshot: snapshot(),
    };
    expect(() => assertUpdateOutcome(outcome)).not.toThrow();
    expect(() => assertUpdateOutcome({ ...outcome, detail: 42 })).toThrow(UpdateValidationError);
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
      client.prepare({
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
      prepare: async () => committed(),
      apply: async () => committed(),
      cancel: async () => committed(),
    });

    await expect(new UpdateClient(port).snapshot()).rejects.toThrow(UpdateValidationError);
  });

  test("reports an early malformed change and closes the late registration exactly once", async () => {
    const port = new Port();
    let released = 0;
    let delivered = 0;
    const failures: Array<{ phase: string; error: unknown }> = [];
    port.listen = async (receive) => {
      receive({ kind: "invalid" });
      receive({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: 3, kind: "checked" });
      return () => {
        released += 1;
      };
    };
    const dispose = await new UpdateClient(port).listen(
      () => {
        delivered += 1;
      },
      (failure) => {
        failures.push(failure);
        throw new Error("reporter failed");
      },
    );

    expect(failures).toEqual([{ phase: "event", error: expect.any(UpdateValidationError) }]);
    expect(delivered).toBe(0);
    expect(released).toBe(1);
    await dispose();
    expect(released).toBe(1);
  });

  test("an early consumer error allows registration to return its unlisten", async () => {
    const port = new Port();
    let emit: ((event: unknown) => void) | undefined;
    port.listen = async (receive) => {
      emit = receive;
      receive(CHANGED_EVENT);
      return () => {
        emit = undefined;
      };
    };
    let delivered = 0;
    const dispose = await new UpdateClient(port).listen(() => {
      delivered += 1;
      if (delivered === 1) {
        throw new Error("early change observer failed");
      }
    });

    emit?.(CHANGED_EVENT);
    expect(delivered).toBe(2);
    await dispose();
    emit?.(CHANGED_EVENT);
    expect(delivered).toBe(2);
  });

  test("a change consumer error propagates without closing the subscription", async () => {
    const port = new Port();
    let emit: ((event: unknown) => void) | undefined;
    port.listen = async (receive) => {
      emit = receive;
      return () => {};
    };
    const error = new Error("change observer failed");
    let delivered = 0;
    const failures: Array<unknown> = [];
    const dispose = await new UpdateClient(port).listen(
      () => {
        delivered += 1;
        throw error;
      },
      (failure) => failures.push(failure),
    );

    expect(() => emit?.(CHANGED_EVENT)).toThrow(error);
    expect(() => emit?.(CHANGED_EVENT)).toThrow(error);
    expect(delivered).toBe(2);
    expect(failures).toEqual([]);
    await dispose();
  });

  test("a malformed progress report is reported, ends delivery and does not throw into the port", async () => {
    const port = new PreparingPort();
    const failures: Array<{ phase: string; error: unknown }> = [];
    let delivered = 0;
    const pending = new UpdateClient(port).prepareWithProgress(
      { protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: 3, version: "1.4.0" },
      () => {
        delivered += 1;
      },
      (failure) => failures.push(failure),
    );
    const transfer = port.transfer(0);

    transfer.report(PROGRESS_EVENT);
    expect(() => transfer.report({ progress: "invalid" })).not.toThrow();
    transfer.report(PROGRESS_EVENT);
    transfer.report({});
    transfer.complete(committed());

    await expect(pending).resolves.toEqual(committed());
    expect(delivered).toBe(1);
    expect(failures).toEqual([{ phase: "event", error: expect.any(UpdateValidationError) }]);
  });

  test("the serialized port survives a structured-clone round trip", async () => {
    const port = new SerializedUpdatePort(new Port());

    await expect(new UpdateClient(port).snapshot()).resolves.toEqual(snapshot());
  });
});

describe("update controller", () => {
  test("a malformed change event closes the subscription and stays visible", async () => {
    const port = new Port();
    let receive: ((event: unknown) => void) | undefined;
    let released = 0;
    port.listen = async (listener) => {
      receive = listener;
      return () => {
        released += 1;
      };
    };
    const controller = new UpdateController({ port });
    await controller.start();

    expect(() => receive?.({})).not.toThrow();
    expect(controller.status).toEqual({
      kind: "failed",
      error: expect.any(UpdateValidationError),
    });
    await controller.refresh();
    expect(controller.status.kind).toBe("failed");
    expect(released).toBe(1);
    await controller.stop();
    expect(released).toBe(1);
  });

  test("a malformed progress report fails the controller until it restarts", async () => {
    const port = new PreparingPort();
    const controller = new UpdateController({ port });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    const transfer = port.transfer(0);
    transfer.report(PROGRESS_EVENT);
    expect(() => transfer.report({})).not.toThrow();
    expect(controller.progress).toEqual({ state: "idle" });
    transfer.complete(committed());
    await expect(pending).resolves.toEqual(committed());

    expect(controller.status).toEqual({
      kind: "failed",
      error: expect.any(UpdateValidationError),
    });
    expect(controller.pending).toBe(false);
    await controller.stop();
    await controller.start();
    expect(controller.status).toEqual({ kind: "ready" });
    // A report from the earlier lifetime cannot fail the new one.
    transfer.report({});
    expect(controller.status).toEqual({ kind: "ready" });
    await controller.stop();
  });

  test.each([
    ["check", (controller: UpdateController) => controller.check()],
    ["apply", (controller: UpdateController) => controller.apply("1.4.0")],
    ["prepare", (controller: UpdateController) => controller.prepare("1.4.0")],
  ])("%s on a stopped controller sends nothing and leaves it idle", async (_name, run) => {
    const port = new Port(snapshot({ staged: STAGED }));
    const controller = new UpdateController({ port });
    await controller.start();
    await controller.stop();
    const calls = [...port.calls];
    let notifications = 0;
    controller.observe(() => {
      notifications += 1;
    });

    await expect(run(controller)).resolves.toEqual({
      status: "failed",
      error: expect.any(UpdateControllerUnavailableError),
    });
    expect(controller.status).toEqual({ kind: "idle" });
    expect(controller.pending).toBe(false);
    expect(port.calls).toEqual(calls);
    expect(notifications).toBe(0);
  });

  test("a command stopped in flight cannot make the stopped controller ready", async () => {
    const port = new PreparingPort();
    const controller = new UpdateController({ port });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    await controller.stop();
    port.transfer(0).complete(committed());
    await pending;

    expect(controller.status).toEqual({ kind: "idle" });
    expect(controller.pending).toBe(false);
  });

  test("applying a version that is not staged fails locally, not as an authority rejection", async () => {
    const port = new Port(snapshot({ staged: STAGED }));
    const controller = new UpdateController({ port });
    await controller.start();

    const result = await controller.apply("9.9.9");

    expect(result).toEqual({ status: "failed", error: expect.any(UpdateNotStagedError) });
    expect(controller.status).toEqual({
      kind: "failed",
      error: expect.any(UpdateNotStagedError),
    });
    expect(controller.lastRejection).toBeUndefined();
    expect(port.calls).not.toContain("apply");
  });

  test("a local apply failure leaves a prepare in flight in charge", async () => {
    const port = new PreparingPort();
    const controller = new UpdateController({ port });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    await controller.apply("1.4.0");
    expect(controller.pending).toBe(true);
    port.transfer(0).complete(committed(snapshot({ staged: STAGED })));
    await pending;

    expect(controller.pending).toBe(false);
    expect(controller.staged).toEqual(STAGED);
    expect(controller.status).toEqual({ kind: "ready" });
    await controller.stop();
  });

  test("refresh after stop leaves the controller idle", async () => {
    const port = new Port();
    const controller = new UpdateController({ port });
    await controller.start();
    const calls = [...port.calls];
    await controller.stop();
    await controller.refresh();

    expect(port.calls).toEqual(calls);
    expect(controller.status).toEqual({ kind: "idle" });
  });

  test("an unlisten failure rejects stop and still restores idle", async () => {
    const failure = new Error("update unlisten failed");
    const port = new Port();
    port.listen = async () => () => {
      throw failure;
    };
    const controller = new UpdateController({ port });
    await controller.start();

    await expect(controller.stop()).rejects.toBe(failure);
    expect(controller.status).toEqual({ kind: "idle" });
    await controller.stop();
  });

  test("a late update refresh cannot replace a command snapshot", async () => {
    const initial = snapshot();
    const updated = snapshot({ installedVersion: "1.4.0" });
    let resolveRefresh!: (state: UpdateSnapshot) => void;
    const slow = new Promise<UpdateSnapshot>((resolve) => {
      resolveRefresh = resolve;
    });
    const port = new Port(initial, () => committed(updated));
    const controller = new UpdateController({ port });
    await controller.start();
    port.snapshot = () => slow;

    const refresh = controller.refresh();
    expect(await controller.check()).toEqual(committed(updated));
    resolveRefresh(initial);
    await refresh;

    expect(controller.snapshot).toEqual(updated);
    expect(controller.status).toEqual({ kind: "ready" });
    await controller.stop();
  });

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
  test("a gated apply stays ready and shows its reason", async () => {
    const cause: DeferralCause = { cause: "workInFlight", detail: "1 open transfer session" };
    const gated = snapshot({
      deferral: { version: "1.4.0", cause },
      staged: STAGED,
      progress: { state: "readyToInstall", version: "1.4.0" },
    });
    const controller = new UpdateController({
      port: new Port(snapshot({ staged: STAGED }), () => committed(gated)),
    });

    await controller.start();
    const result = await controller.apply("1.4.0");

    expect(result).toEqual({
      status: "committedWithDeferral",
      snapshot: gated,
      deferral: { version: "1.4.0", cause },
    });
    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.deferral).toEqual({ version: "1.4.0", cause });
    expect(controller.lastRejection).toBeUndefined();
    expect(controller.installedVersion).toBe("1.3.0");
    // A refused install is not a cancellation: the artifact stays staged.
    expect(controller.staged).toEqual(STAGED);
  });

  test("a rejection carries installer detail to the caller", async () => {
    const controller = new UpdateController({
      port: new Port(snapshot({ staged: STAGED }), () => ({
        status: "rejected",
        code: "notWritable",
        detail: "read-only bundle",
        snapshot: snapshot(),
      })),
    });

    await controller.start();
    const result = await controller.apply("1.4.0");

    expect(result).toEqual({
      status: "rejected",
      code: "notWritable",
      detail: "read-only bundle",
      snapshot: snapshot(),
    });
    expect(controller.status).toEqual({ kind: "ready" });
    expect(controller.lastRejection).toBe("notWritable");
    expect(controller.deferral).toBeUndefined();
  });

  test("a transport failure returns its error and clears the previous rejection", async () => {
    let refuse = true;
    let fail = false;
    const transportError = new Error("connection lost");
    const controller = new UpdateController({
      port: new Port(snapshot(), () => {
        if (fail) {
          throw transportError;
        }
        return refuse
          ? { status: "rejected", code: "unreachable", detail: null, snapshot: snapshot() }
          : committed();
      }),
    });

    await controller.start();
    await controller.prepare("1.4.0");
    expect(controller.lastRejection).toBe("unreachable");

    refuse = false;
    fail = true;
    const result = await controller.check();

    expect(result).toEqual({ status: "failed", error: transportError });
    expect(controller.status).toEqual({ kind: "failed", error: transportError });
    expect(controller.lastRejection).toBeUndefined();
  });

  test("a committed command returns its snapshot", async () => {
    const expected = snapshot();
    const controller = new UpdateController({ port: new Port(expected) });
    await controller.start();

    await expect(controller.check()).resolves.toEqual({ status: "committed", snapshot: expected });
  });

  test("a command sends the epoch the last snapshot carried", async () => {
    const port = new Port(snapshot({ authorityEpoch: 9 }));
    const client = new UpdateClient(port);
    const sent: Array<number> = [];
    const controller = new UpdateController({
      port: {
        snapshot: () => client.snapshot(),
        check: async (command) => {
          sent.push(command.authorityEpoch);
          return committed(snapshot({ authorityEpoch: 9 }));
        },
        selectChannel: async () => committed(),
        defer: async () => committed(),
        prepare: async () => committed(),
        apply: async () => committed(),
        cancel: async () => committed(),
      },
    });

    await controller.start();
    await controller.check();

    expect(sent).toEqual([9]);
  });

  /** Inventing an epoch would have the authority refuse it as stale, which
   * reads as a protocol problem rather than as "nothing has been read yet". */
  test("a command before the first read fails saying so", async () => {
    const port = new Port();
    port.snapshot = () => new Promise(() => {});
    const controller = new UpdateController({ port });
    void controller.start();
    await Promise.resolve();

    await controller.check();

    expect(controller.status.kind).toBe("failed");
    expect(String((controller.status as { error: unknown }).error)).toContain("has not been read");
    await controller.stop();
  });

  test("the staged protocol sends prepare, apply and cancel as their own commands", async () => {
    const port = new Port(snapshot(), (command) =>
      committed(command === "prepare" ? snapshot({ staged: STAGED }) : snapshot()),
    );
    const controller = new UpdateController({ port });

    await controller.start();
    expect((await controller.apply("1.4.0")).status).toBe("failed");
    await controller.prepare("1.4.0");
    await controller.apply("1.4.0");
    await controller.cancel();

    expect(port.calls).toEqual(["listen", "snapshot", "prepare", "apply", "cancel"]);
    expect(port.applied).toEqual([
      {
        protocolVersion: UPDATE_PROTOCOL_VERSION,
        authorityEpoch: 3,
        version: STAGED.version,
        digest: STAGED.digest,
      },
    ]);
  });

  test("stop releases the listener and returns to idle", async () => {
    let released = false;
    const controller = new UpdateController({
      port: createDirectUpdatePort({
        snapshot: async () => snapshot(),
        check: async () => committed(),
        selectChannel: async () => committed(),
        defer: async () => committed(),
        prepare: async () => committed(),
        apply: async () => committed(),
        cancel: async () => committed(),
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

  /**
   * The reason the progress channel exists. A snapshot read during the
   * transfer still shows the retained state, because the host does not hold
   * the authority across the download.
   */
  test("a live report wins over the snapshot while the transfer runs", async () => {
    const port = new PreparingPort();
    const controller = new UpdateController({ port: new SerializedUpdatePort(port) });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    const transfer = port.transfer(0);
    transfer.report({ ...PROGRESS_EVENT, authorityEpoch: 2 });
    expect(controller.progress).toEqual({ state: "idle" });

    transfer.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual(LIVE_PROGRESS);
    expect(controller.presence).toBe("quiet");
    transfer.complete(committed());
    await pending;
  });

  test("a port without invocation-scoped progress prepares without live bytes", async () => {
    const retained = snapshot({ staged: STAGED });
    const port = new Port(snapshot(), () => committed(retained));
    const controller = new UpdateController({ port });
    await controller.start();

    await controller.prepare("1.4.0");

    expect(port.calls).toEqual(["listen", "snapshot", "prepare"]);
    expect(controller.staged).toEqual(STAGED);
    expect(controller.progress).toEqual({ state: "idle" });
    await controller.stop();
  });

  test("the live report is dropped when the command that started it settles", async () => {
    const retained = snapshot({
      staged: STAGED,
      progress: { state: "readyToInstall", version: "1.4.0" },
    });
    const port = new PreparingPort();
    const controller = new UpdateController({ port });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    const transfer = port.transfer(0);
    transfer.report(PROGRESS_EVENT);
    transfer.complete(committed(retained));
    await pending;
    transfer.report(PROGRESS_EVENT);

    expect(controller.progress).toEqual({ state: "readyToInstall", version: "1.4.0" });
    expect(controller.staged).toEqual(STAGED);
  });

  test("drops cancelled and superseded progress even when another prepare uses the same epoch", async () => {
    const port = new PreparingPort();
    const controller = new UpdateController({ port });
    await controller.start();

    const cancelled = controller.prepare("1.4.0");
    const first = port.transfer(0);
    first.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual(LIVE_PROGRESS);

    await controller.cancel();
    first.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });

    const superseded = controller.prepare("1.4.0");
    const second = port.transfer(1);
    first.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });

    const pending = controller.prepare("1.4.0");
    const third = port.transfer(2);
    second.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });
    third.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual(LIVE_PROGRESS);

    first.complete(committed());
    second.complete(committed());
    await Promise.all([cancelled, superseded]);
    expect(controller.progress).toEqual(LIVE_PROGRESS);
    third.complete(committed());
    await pending;
  });

  test("drops old reports after a channel switch and accepts the new epoch", async () => {
    const beta = snapshot({ authorityEpoch: 4, channel: "beta" });
    const port = new PreparingPort(snapshot(), () => committed(beta));
    const controller = new UpdateController({ port });
    await controller.start();

    const previous = controller.prepare("1.4.0");
    const first = port.transfer(0);
    first.report(PROGRESS_EVENT);
    await controller.selectChannel("beta");
    first.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });

    const current = controller.prepare("1.4.0");
    const second = port.transfer(1);
    second.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });
    second.report({ ...PROGRESS_EVENT, authorityEpoch: 4 });
    expect(controller.progress).toEqual(LIVE_PROGRESS);

    first.complete(committed());
    second.complete(committed(beta));
    await Promise.all([previous, current]);
    expect(controller.snapshot?.channel).toBe("beta");
  });

  test("refreshing to a new epoch clears progress from a still pending prepare", async () => {
    let state = snapshot();
    const port = new PreparingPort();
    port.snapshot = async () => state;
    const controller = new UpdateController({ port });
    await controller.start();

    const pending = controller.prepare("1.4.0");
    const transfer = port.transfer(0);
    transfer.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual(LIVE_PROGRESS);

    state = snapshot({ authorityEpoch: 4, channel: "beta" });
    await controller.refresh();
    expect(controller.progress).toEqual({ state: "idle" });
    transfer.report(PROGRESS_EVENT);
    expect(controller.progress).toEqual({ state: "idle" });
    transfer.complete({
      status: "rejected",
      code: "staleAuthority",
      detail: null,
      snapshot: state,
    });
    await pending;
  });
});

async function presence(state: UpdateSnapshot): Promise<string> {
  const controller = new UpdateController({ port: new Port(state) });
  await controller.start();
  return controller.presence;
}

describe("update presence", () => {
  test("nothing to install collapses to nothing", async () => {
    expect(await presence(snapshot())).toBe("hidden");
  });

  /**
   * Three states have a newer version and no install this application can
   * perform. An indicator for any of them offers something that cannot be
   * taken; the settings panel is where they get explained.
   */
  test("withheld, ahead-of-channel and externally managed all stay hidden", async () => {
    const cases: Array<UpdateAvailabilityProjection> = [
      { state: "withheldByRollout", version: "1.4.0" },
      { state: "aheadOfChannel", installed: "1.3.0-nightly.4", channel: "1.2.9" },
      { state: "managedElsewhere", version: "1.4.0", manager: "homebrewCask" },
    ];
    for (const availability of cases) {
      expect(await presence(snapshot({ availability }))).toBe("hidden");
    }
  });

  test("an offer asks for attention", async () => {
    const availability: UpdateAvailabilityProjection = {
      state: "offer",
      version: "1.4.0",
      reason: "staged",
      notes: null,
    };
    expect(await presence(snapshot({ availability }))).toBe("attention");
  });

  /** They said "not now", not "never". An indicator that vanishes gives them
   * no way back to an update they postponed. */
  test("a postponed offer stays, quietly", async () => {
    const availability: UpdateAvailabilityProjection = {
      state: "offer",
      version: "1.4.0",
      reason: "staged",
      notes: null,
    };
    const deferral = { version: "1.4.0", cause: { cause: "userPostponed" } as DeferralCause };
    expect(await presence(snapshot({ availability, deferral }))).toBe("quiet");
  });

  /** A gate deferral is transient -- the offer is still the answer once the
   * transfer finishes -- so it does not quieten the way a decision does. */
  test("a gate deferral does not quieten the offer", async () => {
    const availability: UpdateAvailabilityProjection = {
      state: "offer",
      version: "1.4.0",
      reason: "staged",
      notes: null,
    };
    const cause = { cause: "workInFlight", detail: "1 open transfer" } as DeferralCause;
    expect(await presence(snapshot({ availability, deferral: { version: "1.4.0", cause } }))).toBe(
      "attention",
    );
  });

  test("work in flight is quiet, and ready-to-install asks again", async () => {
    const downloading: UpdateProgressProjection = {
      state: "downloading",
      received: 4096,
      expected: null,
      fraction: null,
    };
    const ready: UpdateProgressProjection = { state: "readyToInstall", version: "1.4.0" };
    expect(await presence(snapshot({ progress: downloading }))).toBe("quiet");
    expect(await presence(snapshot({ progress: { state: "verifying" } }))).toBe("quiet");
    expect(await presence(snapshot({ progress: ready }))).toBe("attention");
  });

  test("presence is hidden before the first read", () => {
    expect(new UpdateController({ port: new Port() }).presence).toBe("hidden");
  });
});
