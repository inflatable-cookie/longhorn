import { describe, expect, it, vi } from "vitest";

import { windowDrag } from "../../src/bindings/index.ts";

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

  it("zooms on a double click, under the same eligibility as the drag", () => {
    const node = document.createElement("header");
    const plain = document.createElement("span");
    const button = document.createElement("button");
    node.append(plain, button);
    const toggleMaximize = vi.fn();
    const errors: unknown[] = [];
    const action = windowDrag(node, {
      startDragging: vi.fn(),
      reportError: (error) => errors.push(error),
      toggleMaximize,
    });

    plain.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    // Everything the drag refuses, the zoom refuses too.
    button.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    for (const init of [
      { button: 1 },
      { button: 0, altKey: true },
      { button: 0, metaKey: true },
      { button: 0, shiftKey: true },
    ]) {
      plain.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, ...init }));
    }

    expect(toggleMaximize).toHaveBeenCalledTimes(1);
    expect(errors).toEqual([]);
    action.destroy();
  });

  it("does nothing on a double click when no zoom is supplied", () => {
    const node = document.createElement("header");
    const startDragging = vi.fn();
    const action = windowDrag(node, { startDragging, reportError: () => {} });

    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));

    expect(startDragging).not.toHaveBeenCalled();
    action.destroy();
  });

  it("reads the zoom at dispatch, so update() can add or remove it", () => {
    const node = document.createElement("header");
    const toggleMaximize = vi.fn();
    const options = { startDragging: vi.fn(), reportError: () => {} };
    const action = windowDrag(node, options);

    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    action.update({ ...options, toggleMaximize });
    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    action.update(options);
    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));

    expect(toggleMaximize).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("stops listening for both gestures once destroyed", () => {
    const node = document.createElement("header");
    const startDragging = vi.fn();
    const toggleMaximize = vi.fn();
    windowDrag(node, {
      startDragging,
      reportError: () => {},
      toggleMaximize,
    }).destroy();

    node.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 }));
    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));

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
      toggleMaximize: () => Promise.reject(failure),
    });

    node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));

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
