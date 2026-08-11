import { describe, expect, it, vi } from "vitest";

import { windowDrag } from "../../src/bindings/index.ts";

// `detail` is the click ordinal the gesture turns on: 1 drags, 2 zooms. jsdom
// does not maintain it across dispatches, so every test states it.
function press(init: MouseEventInit = {}): MouseEvent {
  return new MouseEvent("mousedown", { bubbles: true, button: 0, ...init });
}

function release(init: MouseEventInit = {}): MouseEvent {
  return new MouseEvent("mouseup", { bubbles: true, button: 0, ...init });
}

describe("windowDrag", () => {
  it("starts native drag only for primary unmodified chrome gestures", () => {
    const node = document.createElement("header");
    const plain = document.createElement("span");
    const button = document.createElement("button");
    const link = document.createElement("a");
    const input = document.createElement("input");
    const roleButton = document.createElement("span");
    roleButton.setAttribute("role", "button");
    const optedOut = document.createElement("span");
    optedOut.dataset.noWindowDrag = "";
    node.append(plain, button, link, input, roleButton, optedOut);
    document.body.append(node);
    const startDragging = vi.fn();
    const errors: unknown[] = [];
    const action = windowDrag(node, {
      startDragging,
      reportError: (error) => errors.push(error),
    });

    plain.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    );
    for (const target of [button, link, input, roleButton, optedOut]) {
      target.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, button: 0 }),
      );
    }
    for (const init of [
      { button: 1 },
      { button: 0, altKey: true },
      { button: 0, ctrlKey: true },
      { button: 0, metaKey: true },
      { button: 0, shiftKey: true },
    ]) {
      plain.dispatchEvent(
        new MouseEvent("mousedown", { bubbles: true, ...init }),
      );
    }

    expect(startDragging).toHaveBeenCalledTimes(1);
    expect(errors).toEqual([]);
    action.destroy();
  });

  it("zooms on the second press away from macOS", () => {
    const node = document.createElement("header");
    const plain = document.createElement("span");
    const button = document.createElement("button");
    node.append(plain, button);
    const toggleMaximize = vi.fn();
    const startDragging = vi.fn();
    const action = windowDrag(node, {
      startDragging,
      reportError: () => {},
      zoom: { toggleMaximize, platform: "windows" },
    });

    plain.dispatchEvent(press({ detail: 1 }));
    plain.dispatchEvent(press({ detail: 2 }));

    expect(startDragging).toHaveBeenCalledTimes(1);
    expect(toggleMaximize).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("defers the macOS zoom to pointer up", () => {
    const node = document.createElement("header");
    const toggleMaximize = vi.fn();
    const action = windowDrag(node, {
      startDragging: vi.fn(),
      reportError: () => {},
      zoom: { toggleMaximize, platform: "macOs" },
    });

    node.dispatchEvent(press({ detail: 2, clientX: 40, clientY: 12 }));
    expect(toggleMaximize).not.toHaveBeenCalled();
    node.dispatchEvent(release({ detail: 2, clientX: 40, clientY: 12 }));

    expect(toggleMaximize).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("cancels the macOS zoom when the pointer moves before release", () => {
    const node = document.createElement("header");
    const toggleMaximize = vi.fn();
    const action = windowDrag(node, {
      startDragging: vi.fn(),
      reportError: () => {},
      zoom: { toggleMaximize, platform: "macOs" },
    });

    node.dispatchEvent(press({ detail: 2, clientX: 40, clientY: 12 }));
    node.dispatchEvent(release({ detail: 2, clientX: 41, clientY: 12 }));

    expect(toggleMaximize).not.toHaveBeenCalled();
    action.destroy();
  });

  it("does not zoom on a release that no press armed", () => {
    const node = document.createElement("header");
    const toggleMaximize = vi.fn();
    const action = windowDrag(node, {
      startDragging: vi.fn(),
      reportError: () => {},
      zoom: { toggleMaximize, platform: "macOs" },
    });

    node.dispatchEvent(release({ detail: 2, clientX: 0, clientY: 0 }));
    // A drag press disarms any pending zoom rather than leaving it to fire.
    node.dispatchEvent(press({ detail: 2, clientX: 5, clientY: 5 }));
    node.dispatchEvent(press({ detail: 1, clientX: 5, clientY: 5 }));
    node.dispatchEvent(release({ detail: 2, clientX: 5, clientY: 5 }));

    expect(toggleMaximize).not.toHaveBeenCalled();
    action.destroy();
  });

  it("never drags on the second press, so a zoom cannot also move the window", () => {
    const node = document.createElement("header");
    const startDragging = vi.fn();
    const action = windowDrag(node, {
      startDragging,
      reportError: () => {},
      zoom: { toggleMaximize: vi.fn(), platform: "macOs" },
    });

    node.dispatchEvent(press({ detail: 2 }));

    expect(startDragging).not.toHaveBeenCalled();
    action.destroy();
  });

  it("leaves a double click inert when no zoom is configured", () => {
    const node = document.createElement("header");
    const startDragging = vi.fn();
    const action = windowDrag(node, { startDragging, reportError: () => {} });

    node.dispatchEvent(press({ detail: 2 }));
    node.dispatchEvent(release({ detail: 2 }));

    expect(startDragging).not.toHaveBeenCalled();
    action.destroy();
  });

  it("refuses the zoom under the same exclusions as the drag", () => {
    const node = document.createElement("header");
    const button = document.createElement("button");
    node.append(button);
    const toggleMaximize = vi.fn();
    const action = windowDrag(node, {
      startDragging: vi.fn(),
      reportError: () => {},
      zoom: { toggleMaximize, platform: "windows" },
    });

    button.dispatchEvent(press({ detail: 2 }));
    for (const init of [{ altKey: true }, { metaKey: true }, { shiftKey: true }]) {
      node.dispatchEvent(press({ detail: 2, ...init }));
    }

    expect(toggleMaximize).not.toHaveBeenCalled();
    action.destroy();
  });

  it("reads the zoom at dispatch, so update() can add or remove it", () => {
    const node = document.createElement("header");
    const toggleMaximize = vi.fn();
    const base = { startDragging: vi.fn(), reportError: () => {} };
    const zoom = { toggleMaximize, platform: "windows" as const };
    const action = windowDrag(node, base);

    node.dispatchEvent(press({ detail: 2 }));
    action.update({ ...base, zoom });
    node.dispatchEvent(press({ detail: 2 }));
    action.update(base);
    node.dispatchEvent(press({ detail: 2 }));

    expect(toggleMaximize).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("stops listening for every gesture once destroyed", () => {
    const node = document.createElement("header");
    const startDragging = vi.fn();
    const toggleMaximize = vi.fn();
    windowDrag(node, {
      startDragging,
      reportError: () => {},
      zoom: { toggleMaximize, platform: "windows" },
    }).destroy();

    node.dispatchEvent(press({ detail: 1 }));
    node.dispatchEvent(press({ detail: 2 }));

    expect(startDragging).not.toHaveBeenCalled();
    expect(toggleMaximize).not.toHaveBeenCalled();
  });

  it("routes a failing zoom to the reporter", async () => {
    const node = document.createElement("header");
    const errors: unknown[] = [];
    const failure = new Error("zoom refused");
    windowDrag(node, {
      startDragging: vi.fn(),
      reportError: (error) => errors.push(error),
      zoom: {
        toggleMaximize: () => Promise.reject(failure),
        platform: "windows",
      },
    });

    node.dispatchEvent(press({ detail: 2 }));

    await vi.waitFor(() => expect(errors).toEqual([failure]));
  });

  it("routes synchronous and asynchronous native failures to the reporter", async () => {
    const first = document.createElement("header");
    const second = document.createElement("header");
    const errors: unknown[] = [];
    const syncError = new Error("sync");
    const asyncError = new Error("async");
    windowDrag(first, {
      startDragging: () => {
        throw syncError;
      },
      reportError: (error) => errors.push(error),
    });
    windowDrag(second, {
      startDragging: () => Promise.reject(asyncError),
      reportError: (error) => errors.push(error),
    });

    first.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    );
    second.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    );
    await vi.waitFor(() => expect(errors).toEqual([syncError, asyncError]));
  });
});
