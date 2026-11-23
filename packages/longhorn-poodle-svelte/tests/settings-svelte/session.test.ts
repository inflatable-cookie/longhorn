import { describe, expect, it } from "vitest";

import {
  MissingSettingsRendererError,
  SettingsPageApplyAmbiguityError,
  SettingsSession,
} from "../../src/settings/svelte.ts";
import {
  deferred,
  FakeSettingsTransport,
  fixture,
  registry,
  renderer,
  requestIds,
  twoPageRegistry,
} from "./support.ts";

const intent = {
  codecVersion: 1,
  value: { selected: "device:studio" },
};

describe("SettingsSession", () => {
  it("keeps route and drafts isolated across two instances", async () => {
    const firstTransport = new FakeSettingsTransport();
    const secondTransport = new FakeSettingsTransport();
    const first = new SettingsSession({
      client: firstTransport.client(),
      nextRequestId: requestIds("first"),
    });
    const second = new SettingsSession({
      client: secondTransport.client(),
      nextRequestId: requestIds("second"),
    });

    await Promise.all([first.start(() => renderer()), second.start(() => renderer())]);
    await first.currentContext!.change("app:audio", intent);

    expect(first.dirty).toBe(true);
    expect(second.dirty).toBe(false);
    expect(first.currentContext!.draft("app:audio")?.intent).toEqual(intent);
    expect(second.currentContext!.draft("app:audio")).toBeUndefined();

    await Promise.all([first.stop(), second.stop()]);
  });

  it("loads a shared scope under each requesting page", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue = twoPageRegistry();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("shared-scope"),
    });

    await session.start(() => renderer());
    await session.navigate({ pageId: "app:advanced" });

    const loads = transport.arguments.filter(
      ({ command }) => typeof command === "object" && command !== null && "scopeId" in command,
    );
    expect(loads).toEqual([
      {
        command: expect.objectContaining({
          pageId: "app:audio",
          scopeId: "app:preferences",
        }),
      },
      {
        command: expect.objectContaining({
          pageId: "app:advanced",
          scopeId: "app:preferences",
        }),
      },
    ]);
    await session.stop();
  });

  it("guards dirty navigation and close until apply, discard, or stay", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue = twoPageRegistry();
    let closes = 0;
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("guard"),
      onClose: () => {
        closes += 1;
      },
    });
    await session.start(() => renderer());
    await session.currentContext!.change("app:audio", intent);

    await expect(session.navigate({ pageId: "app:advanced" })).resolves.toBe(false);
    expect(session.guard?.kind).toBe("navigate");
    expect(session.route?.pageId).toBe("app:audio");
    await expect(session.resolveGuard("stay")).resolves.toBe(false);
    expect(session.guard).toBeUndefined();

    expect(session.requestClose()).toBe(false);
    await expect(session.resolveGuard("discard")).resolves.toBe(true);
    expect(closes).toBe(1);
    expect(session.dirty).toBe(false);
    await session.stop();
  });

  it("preserves a staged draft and exposes fresh authority on conflict", async () => {
    const transport = new FakeSettingsTransport();
    transport.mutationValue = structuredClone(
      fixture.mutationResults[2],
    ) as typeof transport.mutationValue;
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("conflict"),
    });
    await session.start(() => renderer());
    await session.currentContext!.change("app:audio", intent);
    await session.currentContext!.apply("app:audio");

    expect(session.dirty).toBe(true);
    expect(session.primaryUnitStatus.kind).toBe("conflict");
    expect(session.scopeSnapshot("app:preferences")?.authority.authorityToken).toBe(
      "authority:current",
    );
    await session.stop();
  });

  it("never reports Saved when an immediate mutation fails", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue = registry("immediate");
    const failure = new Error("authority offline");
    transport.mutationError = failure;
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("immediate"),
    });
    await session.start(() => renderer());

    await expect(session.currentContext!.change("app:audio", intent)).rejects.toBe(failure);
    expect(session.primaryUnitStatus).toEqual({
      kind: "failed",
      error: failure,
    });
    await session.stop();
  });

  it("makes an authoritative scope load failure visible at session level", async () => {
    const transport = new FakeSettingsTransport();
    const failure = new Error("scope unavailable");
    const errors: Array<unknown> = [];
    transport.loadError = failure;
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("scope-failure"),
      onError: (error) => errors.push(error),
    });

    await expect(session.start(() => renderer())).rejects.toBe(failure);
    expect(session.status).toEqual({ kind: "failed", error: failure });
    expect(errors).toEqual([failure]);
    await session.stop();
  });

  it("keeps activation separate from persistence across route changes", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue = twoPageRegistry();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("activation"),
    });
    await session.start(() => renderer());
    await session.currentContext!.change("app:audio", intent);
    await session.applyCurrent();

    expect(session.primaryUnitStatus.kind).toBe("saved");
    expect(session.activationRequirements).toContainEqual({
      targetId: "activation:app",
      state: "pending",
    });
    await session.navigate({ pageId: "app:advanced" });
    expect(session.activationRequirements).toContainEqual({
      targetId: "activation:app",
      state: "pending",
    });
    await session.stop();
  });

  it("fails before reveal when a registered renderer is missing", async () => {
    const transport = new FakeSettingsTransport();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("missing"),
    });

    await expect(session.start(() => undefined)).rejects.toBeInstanceOf(
      MissingSettingsRendererError,
    );
    expect(session.status.kind).toBe("failed");
    expect(session.currentPage).toBeUndefined();
    await session.stop();
  });

  it("tears down listener registration that resolves after stop", async () => {
    const transport = new FakeSettingsTransport();
    transport.deferRegistryListener = true;
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("late"),
    });
    const start = session.start(() => renderer());
    const stop = session.stop();
    transport.releaseRegistryListener();

    await expect(start).rejects.toThrow(/disposed/);
    await stop;
    expect(transport.unlistenCount).toBe(1);
    expect(transport.activeListenerCount()).toBe(0);
    expect(session.status.kind).toBe("idle");
  });

  it("exposes unsupported, recovery, and reconnecting states explicitly", async () => {
    const unsupportedTransport = new FakeSettingsTransport();
    unsupportedTransport.registryValue.pages = [];
    const unsupported = new SettingsSession({
      client: unsupportedTransport.client(),
      nextRequestId: requestIds("unsupported"),
    });
    await unsupported.start(() => renderer());
    expect(unsupported.status.kind).toBe("unsupported");
    await unsupported.stop();

    const transport = new FakeSettingsTransport();
    transport.scopeValue.recovery = {
      code: "recoveryRequired",
      diagnostic: null,
    };
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("reconnect"),
    });
    await session.start(() => renderer());
    expect(session.recovery?.code).toBe("recoveryRequired");

    const gate = deferred();
    transport.loadGate = gate.promise;
    const reconnect = session.reconnect();
    expect(session.status.kind).toBe("reconnecting");
    gate.resolve();
    await reconnect;
    expect(session.status.kind).toBe("ready");
    await session.stop();
  });

  it("does not imply page atomicity across multiple dirty units", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue.applyUnits.push({
      ...transport.registryValue.applyUnits[0]!,
      id: "app:advanced-audio",
    });
    transport.registryValue.pages[0]!.writableApplyUnitIds.push("app:advanced-audio");
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("multi"),
    });
    await session.start(() => renderer());
    await session.currentContext!.change("app:audio", intent);
    await session.currentContext!.change("app:advanced-audio", intent);

    await expect(session.applyCurrent()).rejects.toBeInstanceOf(SettingsPageApplyAmbiguityError);
    expect(session.draftCount).toBe(2);
    await session.stop();
  });

  /**
   * Clearing authority empties the scope map before it waits for the scope
   * subscriptions to release. Two registry changes can therefore arrive with
   * the second clear finding nothing to dispose: it installed its registry
   * first and was then overwritten when the older clear finished, leaving the
   * document a generation behind. The older replacement now waits its turn.
   */
  it("keeps the newest registry when two changes overlap a paused disposal", async () => {
    const newer = fixture.registry.generation + 1;
    const newest = fixture.registry.generation + 2;
    const transport = new FakeSettingsTransport();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("replace"),
    });
    await session.start(() => renderer());
    expect(session.registry?.generation).toBe(fixture.registry.generation);

    const disposal = transport.deferNextScopeUnlisten();
    transport.publishRegistry(newer);
    // The first replacement is now disposing the outgoing scopes and cannot
    // finish until the gate below releases.
    await disposal.entered;
    transport.publishRegistry(newest);
    await transport.drain();
    disposal.release();

    await transport.settle(() => session.status.kind === "ready");
    expect(session.registry?.generation).toBe(newest);
    expect(session.status.kind).toBe("ready");
    await session.stop();
  });

  /**
   * A stopped session's install turn must not swallow a restarted session's
   * queue. The turn belongs to the session that started it, so a change queued
   * after the restart is a new turn's work; handing it the stopped session's
   * completion leaves it queued with no turn left to install it, and the
   * document stays on the registry it already had.
   */
  it("installs a change queued after a restart over a stopped session's drain", async () => {
    const newer = fixture.registry.generation + 1;
    const newest = fixture.registry.generation + 2;
    const transport = new FakeSettingsTransport();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("restart"),
    });
    await session.start(() => renderer());

    // Hold a replacement's scope disposal open, then stop the session while
    // that turn is still waiting to clear authority.
    const disposal = transport.deferNextScopeUnlisten();
    transport.publishRegistry(newer);
    await disposal.entered;
    await session.stop();

    await session.start(() => renderer());
    expect(session.registry?.generation).toBe(newer);

    transport.publishRegistry(newest);
    await transport.settle(
      () => session.registry?.generation === newest && session.status.kind === "ready",
    );
    // The abandoned turn finishes only now; it must not take the queue with it.
    disposal.release();
    await transport.drain();

    expect(session.registry?.generation).toBe(newest);
    expect(session.status.kind).toBe("ready");
    await session.stop();
  });

  it("ignores a mutation result that arrives after teardown", async () => {
    const transport = new FakeSettingsTransport();
    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("pending"),
    });
    await session.start(() => renderer());
    await session.currentContext!.change("app:audio", intent);
    const gate = deferred();
    transport.mutationGate = gate.promise;
    const mutation = session.applyCurrent();
    expect(session.busy).toBe(true);

    await session.stop();
    expect(session.status.kind).toBe("idle");
    expect(transport.activeListenerCount()).toBe(0);
    gate.resolve();
    await mutation;
    expect(session.status.kind).toBe("idle");
  });
});
