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

describe("agent-control setFileInput", () => {
  const hi = {
    name: "manifest.json",
    mediaType: "application/json",
    contentBase64: Buffer.from('{"ok":true}', "utf8").toString("base64"),
  };

  test("assigns files, fires input/change, and file.text() returns the content", async () => {
    const window = openPage(
      `<label for="manifest">Manifest</label><input id="manifest" type="file" />`,
    );
    const field = window.document.getElementById("manifest") as unknown as {
      files: ArrayLike<{ name: string; type: string; text: () => Promise<string> }>;
      addEventListener: (type: string, listener: () => void) => void;
    };
    const seen = { input: false, change: false };
    field.addEventListener("input", () => {
      seen.input = true;
    });
    let read: Promise<{ name: string; type: string; text: string }> | undefined;
    field.addEventListener("change", () => {
      seen.change = true;
      const file = field.files[0];
      read = file.text().then((text) => ({ name: file.name, type: file.type, text }));
    });
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const target = findByName(snapshot.root, "Manifest");
    expect(api.setFileInput(target!.elementRef, [hi]).ok).toBe(true);
    expect(seen.input).toBe(true);
    expect(seen.change).toBe(true);
    expect(read).toBeDefined();
    await expect(read!).resolves.toEqual({
      name: "manifest.json",
      type: "application/json",
      text: '{"ok":true}',
    });
  });

  test("rejects a non-file target", () => {
    const window = openPage(`<button id="go">Go</button>`);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const go = findByName(snapshot.root, "Go");
    const result = api.setFileInput(go!.elementRef, [hi]);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error).toEqual({
      error: "unsupported",
      message: "set_file_input requires an input type=file",
    });
  });

  test("rejects a stale ref", () => {
    const window = openPage(`<input type="file" />`);
    const api = install(window);
    const result = api.setFileInput("missing", [hi]);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error).toEqual({ error: "unresolvedRef", element: "missing" });
  });

  test("rejects more than one file unless multiple", () => {
    const window = openPage(
      `<label for="one">One</label><input id="one" type="file" />`,
    );
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const one = findByName(snapshot.root, "One");
    const result = api.setFileInput(one!.elementRef, [
      hi,
      { ...hi, name: "other.json" },
    ]);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.error).toBe("unsupported");
    if (result.error.error !== "unsupported") return;
    expect(result.error.message).toContain("multiple");
  });

  test("accepts multiple files when the input allows it", async () => {
    const window = openPage(
      `<label for="many">Many</label><input id="many" type="file" multiple />`,
    );
    const field = window.document.getElementById("many") as unknown as {
      files: ArrayLike<{ name: string }>;
    };
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const many = findByName(snapshot.root, "Many");
    expect(
      api.setFileInput(many!.elementRef, [hi, { ...hi, name: "other.json" }]).ok,
    ).toBe(true);
    expect(field.files.length).toBe(2);
    expect(field.files[0]?.name).toBe("manifest.json");
    expect(field.files[1]?.name).toBe("other.json");
  });

  test("rejects an accept mismatch", () => {
    const window = openPage(
      `<label for="json">Json</label><input id="json" type="file" accept=".json,application/json" />`,
    );
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const json = findByName(snapshot.root, "Json");
    const result = api.setFileInput(json!.elementRef, [
      { name: "photo.png", mediaType: "image/png", contentBase64: hi.contentBase64 },
    ]);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.error).toBe("unsupported");
    if (result.error.error !== "unsupported") return;
    expect(result.error.message).toContain("accept");
  });

  test("accepts a matching extension and media-type wildcard", () => {
    const window = openPage(
      `<label for="pics">Pics</label><input id="pics" type="file" accept="image/*,.json" />`,
    );
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const pics = findByName(snapshot.root, "Pics");
    expect(
      api.setFileInput(pics!.elementRef, [
        { name: "photo.PNG", mediaType: "image/png", contentBase64: hi.contentBase64 },
      ]).ok,
    ).toBe(true);
    expect(
      api.setFileInput(pics!.elementRef, [
        { name: "data.json", mediaType: "application/json", contentBase64: hi.contentBase64 },
      ]).ok,
    ).toBe(true);
  });

  test("rejects bad base64 in the page", () => {
    const window = openPage(
      `<label for="manifest">Manifest</label><input id="manifest" type="file" />`,
    );
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const target = findByName(snapshot.root, "Manifest");
    const result = api.setFileInput(target!.elementRef, [
      { name: "manifest.json", contentBase64: "@@@" },
    ]);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.error).toBe("unsupported");
    if (result.error.error !== "unsupported") return;
    expect(result.error.message).toContain("base64");
  });

  test("rejects an empty file list", () => {
    const window = openPage(
      `<label for="manifest">Manifest</label><input id="manifest" type="file" />`,
    );
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const target = findByName(snapshot.root, "Manifest");
    const result = api.setFileInput(target!.elementRef, []);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.error).toBe("unsupported");
    if (result.error.error !== "unsupported") return;
    expect(result.error.message).toContain("at least one file");
  });
});
