import { describe, expect, test } from "bun:test";

import { findByName, install, openPage } from "./support.ts";

function dispatchTrusted(window: ReturnType<typeof openPage>, type: string): void {
  const event = new window.Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "isTrusted", { value: true });
  window.dispatchEvent(event);
}

describe("agent-control picker origin", () => {
  test("starts human so an active shim does not capture a picker", () => {
    const window = openPage(`<button>Go</button>`);
    const api = install(window);
    expect(api.isAgentOriginated()).toBe(false);
  });

  test("synthetic click marks agent origin", () => {
    const window = openPage(`<button>Go</button>`);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const go = findByName(snapshot.root, "Go");
    expect(api.click(go!.elementRef).ok).toBe(true);
    expect(api.isAgentOriginated()).toBe(true);
  });

  test("trusted input after a synthetic click restores the human path", () => {
    const window = openPage(`<button>Go</button>`);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    expect(api.click(findByName(snapshot.root, "Go")!.elementRef).ok).toBe(true);
    expect(api.isAgentOriginated()).toBe(true);
    dispatchTrusted(window, "click");
    expect(api.isAgentOriginated()).toBe(false);
  });

  test("synthetic click after trusted input is agent-originated", () => {
    const window = openPage(`<button>Go</button>`);
    const api = install(window);
    dispatchTrusted(window, "pointerdown");
    expect(api.isAgentOriginated()).toBe(false);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    expect(api.click(findByName(snapshot.root, "Go")!.elementRef).ok).toBe(true);
    expect(api.isAgentOriginated()).toBe(true);
  });

  test("evaluate-style markAgentOrigin does not require a click", () => {
    const window = openPage(`<p>Idle</p>`);
    const api = install(window);
    api.markAgentOrigin();
    expect(api.isAgentOriginated()).toBe(true);
  });
});
