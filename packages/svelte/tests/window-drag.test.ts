import { describe, expect, it, vi } from "vitest";

import { windowDrag } from "../src/index.ts";

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
