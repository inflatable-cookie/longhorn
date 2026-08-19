import { describe, expect, test } from "bun:test";

import { findByName, install, openPage } from "./support.ts";

describe("agent-control synthetic input", () => {
  test("click dispatches the pointer/mouse sequence onto real handlers", () => {
    const window = openPage(`<button id="go">Go</button><span id="out"></span>`);
    const button = window.document.getElementById("go")!;
    const seen: string[] = [];
    for (const type of ["pointerdown", "mousedown", "pointerup", "mouseup", "click"]) {
      button.addEventListener(type, () => seen.push(type));
    }
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const go = findByName(snapshot.root, "Go");
    expect(api.click(go!.elementRef).ok).toBe(true);
    expect(seen).toEqual(["pointerdown", "mousedown", "pointerup", "mouseup", "click"]);
    expect(window.document.activeElement).toBe(button as unknown as typeof window.document.activeElement);
  });

  test("type reaches key handlers and value listeners", () => {
    const window = openPage(`<label for="field">Note</label><input id="field" />`);
    const field = window.document.getElementById("field") as unknown as {
      value: string;
      addEventListener: (type: string, listener: (event: Event) => void) => void;
    };
    const keys: string[] = [];
    const inputs: string[] = [];
    let changed = false;
    field.addEventListener("keydown", (event) => keys.push((event as KeyboardEvent).key));
    field.addEventListener("input", () => inputs.push(field.value));
    field.addEventListener("change", () => {
      changed = true;
    });
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const note = findByName(snapshot.root, "Note");
    expect(api.type(note!.elementRef, "hi").ok).toBe(true);
    expect(field.value).toBe("hi");
    expect(keys).toEqual(["h", "i"]);
    expect(inputs).toEqual(["h", "hi"]);
    expect(changed).toBe(true);
  });

  test("press sends modifiers to the focused element", () => {
    const window = openPage(`<input id="field" />`);
    const field = window.document.getElementById("field") as unknown as {
      focus: () => void;
      addEventListener: (type: string, listener: (event: Event) => void) => void;
    };
    field.focus();
    let received: { key: string; meta: boolean; shift: boolean } | undefined;
    field.addEventListener("keydown", (event) => {
      const key = event as KeyboardEvent;
      received = { key: key.key, meta: key.metaKey, shift: key.shiftKey };
    });
    const api = install(window);
    expect(api.press("Enter", ["meta", "shift"]).ok).toBe(true);
    expect(received).toEqual({ key: "Enter", meta: true, shift: true });
  });

  test("scroll updates the target element's scroll position", () => {
    const window = openPage(
      `<div id="box" style="height:40px;overflow:auto"><div style="height:400px">Tall</div></div>`,
    );
    const box = window.document.getElementById("box")!;
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    // The scroller itself may be flattened; stamp it directly if needed.
    box.setAttribute("data-longhorn-agent-ref", "scroller");
    expect(api.scroll(0, 80, "scroller").ok).toBe(true);
    expect(box.scrollTop).toBe(80);
  });

  test("drag dispatches the untrusted DOM sequence", () => {
    const window = openPage(`<div id="src">Src</div><div id="dst">Dst</div>`);
    const src = window.document.getElementById("src")!;
    const dst = window.document.getElementById("dst")!;
    src.setAttribute("data-longhorn-agent-ref", "src");
    dst.setAttribute("data-longhorn-agent-ref", "dst");
    const seen: string[] = [];
    src.addEventListener("dragstart", () => seen.push("dragstart"));
    dst.addEventListener("dragover", () => seen.push("dragover"));
    dst.addEventListener("drop", () => seen.push("drop"));
    src.addEventListener("dragend", () => seen.push("dragend"));
    const api = install(window);
    expect(api.drag("src", "dst").ok).toBe(true);
    expect(seen).toEqual(["dragstart", "dragover", "drop", "dragend"]);
  });
});
