import { describe, expect, it } from "vitest";

import { SettingsSession } from "../../src/settings/svelte.ts";
import {
  deferred,
  FakeSettingsTransport,
  registry,
  renderer,
  requestIds,
} from "./support.ts";

/** Waits for a condition across microtasks and timers, without a DOM. */
async function until(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  throw new Error("condition was never met");
}

/** A page with a renderer and no durable preference scope or apply unit. */
function rendererOnlyPage(value: ReturnType<typeof registry>) {
  return {
    ...structuredClone(value.pages[0]!),
    id: "app:renderer-only",
    label: "Renderer only",
    anchors: [],
    requiredCapabilities: [],
    readableScopeIds: [],
    writableApplyUnitIds: [],
  };
}

describe("renderer-only settings pages", () => {
  it("creates a page session and render context for a page with no scopes or apply units", async () => {
    const transport = new FakeSettingsTransport();
    const value = registry();
    for (const page of value.pages) {
      (page as { readableScopeIds: string[] }).readableScopeIds = [];
      (page as { writableApplyUnitIds: string[] }).writableApplyUnitIds = [];
    }
    transport.registryValue = value;

    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("renderer-only"),
    });
    await session.start(() => renderer());

    const pageId = value.pages[0]!.id;
    await session.navigate({ pageId });

    expect(session.currentPage?.id).toBe(pageId);
    expect(session.currentRenderer).toBeDefined();
    expect(session.currentPageSession).toBeDefined();
    expect(session.currentContext).toBeDefined();
  });

  // A route with scopes starts them asynchronously before it commits, so an
  // earlier route can settle after a later one. A renderer-only page has no
  // scopes, so it commits immediately — and the page it was navigated away
  // from then overwrote it. Last navigation wins.
  it("keeps the latest navigation while an earlier route's scopes are still starting", async () => {
    const transport = new FakeSettingsTransport();
    const value = registry();
    const scoped = value.pages[0]!;
    value.pages.push(rendererOnlyPage(value));
    transport.registryValue = value;
    const gate = deferred();
    transport.loadGate = gate.promise;

    const session = new SettingsSession({
      client: transport.client(),
      nextRequestId: requestIds("route-race"),
      initialRoute: { pageId: scoped.id },
    });
    void session.start(() => renderer());

    // The initial route's scope is gated; navigate away meanwhile.
    await until(() => session.registry !== undefined);
    expect(await session.navigate({ pageId: "app:renderer-only" })).toBe(true);
    expect(session.currentPage?.id).toBe("app:renderer-only");

    // Releasing the first route's scope must not undo the navigation.
    gate.resolve();
    await until(() => transport.activeListenerCount() >= 0);
    await new Promise((resolve) => setTimeout(resolve, 5));
    expect(session.currentPage?.id).toBe("app:renderer-only");
    expect(session.currentContext).toBeDefined();

    await session.stop();
  });
});
