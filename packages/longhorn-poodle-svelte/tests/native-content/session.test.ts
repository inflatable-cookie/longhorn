import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  NativeContentSession,
  resolveNativeContentVisibility,
} from "../../src/native-content/index.ts";
import PoodleLayoutSeamHarness from "./PoodleLayoutSeamHarness.svelte";
import {
  ResizeObserverTrace,
  ScriptedClient,
  committedResult,
  deferred,
  viewportElement,
} from "./support.ts";

function session(
  client: ScriptedClient,
  observers: ResizeObserverTrace,
  inputRouting: "native_direct" | "renderer_forwarded" = "native_direct",
): NativeContentSession {
  return new NativeContentSession({
    client,
    scale: 2000,
    visibility: { state: "visible" },
    focus: "unchanged",
    inputRouting,
    resizeObserverFactory: observers.factory,
  });
}

describe("NativeContentSession", () => {
  it("keeps child-view and backing-surface instances isolated", async () => {
    const childClient = new ScriptedClient();
    const backingClient = new ScriptedClient();
    const childObservers = new ResizeObserverTrace();
    const backingObservers = new ResizeObserverTrace();
    const child = session(childClient, childObservers, "native_direct");
    const backing = session(
      backingClient,
      backingObservers,
      "renderer_forwarded",
    );
    child.bindViewport(
      viewportElement({ left: 10, top: 20, width: 640, height: 360 }),
    );
    backing.bindViewport(
      viewportElement({ left: 30, top: 40, width: 800, height: 450 }),
    );

    await Promise.all([child.start(), backing.start()]);
    await Promise.all([child.whenSettled(), backing.whenSettled()]);

    expect(childClient.connections).toHaveLength(1);
    expect(backingClient.connections).toHaveLength(1);
    expect(childClient.connections[0].updates.at(-1)).toMatchObject({
      generation: child.snapshot?.cursor.attach_generation,
      viewport: {
        origin: { x: 10, y: 20 },
        size: { width: 640, height: 360 },
      },
      scale: 2000,
      input_routing: "native_direct",
    });
    expect(backingClient.connections[0].updates.at(-1)).toMatchObject({
      viewport: {
        origin: { x: 30, y: 40 },
        size: { width: 800, height: 450 },
      },
      input_routing: "renderer_forwarded",
    });

    await child.stop();
    expect(childClient.connections[0].disposeCount).toBe(1);
    expect(backing.status.kind).toBe("ready");
    expect(backingClient.connections[0].disposeCount).toBe(0);
    await backing.stop();
  });

  it("serializes resize, scale, visibility, focus, and input changes", async () => {
    const client = new ScriptedClient();
    const observers = new ResizeObserverTrace();
    const state = session(client, observers);
    const element = viewportElement({
      left: 12,
      top: 16,
      width: 640,
      height: 360,
    });
    state.bindViewport(element);
    await state.start();
    await state.whenSettled();

    const connection = client.connections[0];
    connection.updates.length = 0;
    const release = connection.blockNextUpdate();
    state.setScale(1500);
    await Promise.resolve();

    element.getBoundingClientRect = () =>
      ({
        left: 22,
        top: 28,
        width: 700,
        height: 400,
        x: 22,
        y: 28,
        right: 722,
        bottom: 428,
        toJSON: () => ({}),
      }) as DOMRect;
    observers.fire();
    state.setVisibilityPolicy({ state: "hidden", reason: "dialog_open" });
    state.setFocusIntent("request");
    state.setInputRouting("renderer_forwarded");

    expect(connection.updates).toHaveLength(1);
    release();
    await state.whenSettled();

    expect(connection.updates).toHaveLength(2);
    expect(connection.updates[1]).toMatchObject({
      viewport: {
        origin: { x: 22, y: 28 },
        size: { width: 700, height: 400 },
      },
      scale: 1500,
      visibility: { state: "hidden", reason: "dialog_open" },
      focus: "request",
      input_routing: "renderer_forwarded",
    });
    await state.stop();
  });

  it("rejects a late generation result and replans from current authority", async () => {
    const client = new ScriptedClient();
    const observers = new ResizeObserverTrace();
    const state = session(client, observers);
    state.bindViewport(
      viewportElement({ left: 0, top: 0, width: 500, height: 300 }),
    );
    await state.start();
    await state.whenSettled();

    const connection = client.connections[0];
    const late = deferred<ReturnType<typeof committedResult>>();
    connection.resolveNextUpdateWith(late.promise);
    state.setScale(1750);
    await Promise.resolve();
    const generationTwo = connection.advanceGeneration(2);

    const stale = generationTwo;
    stale.cursor.attach_generation = 1;
    stale.desired.generation = 1;
    late.resolve(committedResult(stale));
    await state.whenSettled();

    expect(state.snapshot?.cursor.attach_generation).toBe(2);
    expect(connection.updates.at(-1)?.generation).toBe(2);
    expect(connection.updates.at(-1)?.scale).toBe(1750);
    await state.stop();
  });

  it("invalidates pending work across unmount and remount", async () => {
    const client = new ScriptedClient();
    const observers = new ResizeObserverTrace();
    const state = session(client, observers);
    state.bindViewport(
      viewportElement({ left: 4, top: 8, width: 320, height: 180 }),
    );
    await state.start();
    await state.whenSettled();

    const first = client.connections[0];
    const release = first.blockNextUpdate();
    state.setScale(1250);
    await Promise.resolve();
    await state.stop();
    await state.start();

    expect(client.connections).toHaveLength(2);
    expect(first.disposeCount).toBe(1);
    release();
    await state.whenSettled();
    expect(state.snapshot?.cursor.client_epoch).toBe(2);
    expect(state.status.kind).toBe("ready");
    expect(client.connections[1].updates.at(-1)?.scale).toBe(1250);
    await state.stop();
  });

  it("composes through public Poodle children and tears down exactly", async () => {
    const client = new ScriptedClient();
    const observers = new ResizeObserverTrace();
    const state = session(client, observers, "renderer_forwarded");
    const mounted = render(PoodleLayoutSeamHarness, {
      props: { session: state, mechanism: "backing_surface" },
    });

    await waitFor(() => expect(state.status.kind).toBe("ready"));
    expect(mounted.getByRole("region", { name: "Native renderer" })).toBeTruthy();
    expect(
      mounted.getByTestId("consumer-native-viewport").getAttribute(
        "data-native-mechanism",
      ),
    ).toBe("backing_surface");

    await mounted.unmount();
    await waitFor(() => expect(client.connections[0].disposeCount).toBe(1));
    expect(client.connections[0].listenerAttached).toBe(false);
    expect(observers.observers[0].disconnectCount).toBe(1);

    const remounted = render(PoodleLayoutSeamHarness, {
      props: { session: state, mechanism: "child_view" },
    });
    await waitFor(() => expect(client.connections).toHaveLength(2));
    await remounted.unmount();
    await waitFor(() => expect(client.connections[1].disposeCount).toBe(1));
    expect(client.connections[1].listenerAttached).toBe(false);
    expect(observers.observers[1].disconnectCount).toBe(1);
  });

  it("resolves only explicit consumer visibility inhibitors", () => {
    expect(
      resolveNativeContentVisibility([
        { reason: "tab_inactive", active: false },
        { reason: "dialog_open", active: true },
        { reason: "window_hidden", active: true },
      ]),
    ).toEqual({ state: "hidden", reason: "dialog_open" });
    expect(resolveNativeContentVisibility([])).toEqual({ state: "visible" });
  });
});
