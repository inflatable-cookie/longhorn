// Browser behaviour the shim meets in a page, exercised against happy-dom.
// None of it has run in a live webview.
import assert from "node:assert/strict";
import { describe, expect, test } from "bun:test";

import { collectRoles, install, openPage } from "./support.ts";

interface Field {
  value: string;
  textContent: string | null;
  setAttribute: (name: string, value: string) => void;
  addEventListener: (type: string, listener: (event: Event) => void) => void;
}

/** One element, stamped with the ref the shim will resolve. */
function page(html: string) {
  const window = openPage(html);
  const field = window.document.getElementById("field") as unknown as Field;
  field.setAttribute("data-longhorn-agent-ref", "field");
  return { window, field, api: install(window) };
}

describe("agent-control typing into inputs without a selection", () => {
  test.each([
    ["email", "ada@example.test"],
    ["number", "42"],
    ["date", "2026-11-28"],
    ["time", "09:30"],
  ])("type=%s takes the whole text instead of throwing", (type, text) => {
    const { field, api } = page(`<input id="field" type="${type}" />`);
    let changes = 0;
    field.addEventListener("change", () => {
      changes += 1;
    });

    expect(api.type("field", text)).toHaveProperty("ok", true);
    expect(field.value).toBe(text);
    expect(changes).toBe(1);
  });

  test("a value the input blanks until it is complete is not lost between keys", () => {
    const { field, api } = page(`<input id="field" type="number" />`);
    // A browser blanks "-" and "1." until the number is complete.
    let value = "";
    Object.defineProperty(field, "value", {
      get: () => value,
      set: (next: string) => {
        value = /^-?\d+(\.\d+)?$/.test(next) ? next : "";
      },
    });

    expect(api.type("field", "-1.5")).toHaveProperty("ok", true);
    expect(field.value).toBe("-1.5");
  });

  test("an email field appends, and follows a rewrite the page makes between keys", () => {
    const { field, api } = page(`<input id="field" type="email" value="a" />`);
    field.addEventListener("input", () => {
      if (field.value === "ab") {
        field.value = "AB";
      }
    });

    expect(api.type("field", "bc")).toHaveProperty("ok", true);
    expect(field.value).toBe("ABc");
  });

  test("a temporal field is replaced, not appended to", () => {
    const { field, api } = page(`<input id="field" type="date" value="2025-01-01" />`);

    expect(api.type("field", "2026-11-28")).toHaveProperty("ok", true);
    expect(field.value).toBe("2026-11-28");
  });

  test("keys reach a checkbox without rewriting its value", () => {
    const { field, api } = page(`<input id="field" type="checkbox" value="yes" />`);
    const seen: Array<string> = [];
    for (const type of ["keydown", "input", "change", "keyup"]) {
      field.addEventListener(type, () => seen.push(type));
    }

    expect(api.type("field", " ")).toHaveProperty("ok", true);
    expect(field.value).toBe("yes");
    expect(seen).toEqual(["keydown", "keyup"]);
  });

  test("a file input answers unsupported instead of throwing", () => {
    const { api } = page(`<input id="field" type="file" />`);

    expect(api.type("field", "/etc/hosts")).toEqual({
      ok: false,
      error: {
        error: "unsupported",
        message: "type cannot set an input type=file; use set_file_input",
      },
    });
  });
});

describe("agent-control cancelled keydown", () => {
  test("suppresses the character and its input event, and still releases the key", () => {
    const { field, api } = page(`<input id="field" />`);
    const seen: Array<string> = [];
    field.addEventListener("keydown", (event) => {
      if ((event as KeyboardEvent).key === "x") {
        event.preventDefault();
      }
    });
    for (const type of ["input", "keyup", "change"]) {
      field.addEventListener(type, () => seen.push(type));
    }

    expect(api.type("field", "axb")).toHaveProperty("ok", true);
    expect(field.value).toBe("ab");
    expect(seen).toEqual(["input", "keyup", "keyup", "input", "keyup", "change"]);
  });

  test("a field that cancels every key reports no change", () => {
    const { field, api } = page(`<input id="field" />`);
    let changes = 0;
    field.addEventListener("keydown", (event) => event.preventDefault());
    field.addEventListener("change", () => {
      changes += 1;
    });

    expect(api.type("field", "hi")).toHaveProperty("ok", true);
    expect(field.value).toBe("");
    expect(changes).toBe(0);
  });
});

describe("agent-control contenteditable", () => {
  test.each(["", "true", "plaintext-only", "TRUE"])(
    'contenteditable="%s" is a textbox',
    (value) => {
      const { field, api } = page(`<div id="field" contenteditable="${value}">a</div>`);
      const snapshot = api.snapshot();
      assert(snapshot.ok);

      expect(collectRoles(snapshot.root)).toContain("textbox");
      expect(api.type("field", "b")).toHaveProperty("ok", true);
      expect(field.textContent).toBe("ab");
    },
  );

  test('contenteditable="false" is neither a textbox nor typed into', () => {
    const { field, api } = page(`<div id="field" contenteditable="false">a</div>`);
    const snapshot = api.snapshot();
    assert(snapshot.ok);

    expect(collectRoles(snapshot.root)).not.toContain("textbox");
    expect(api.type("field", "b")).toHaveProperty("ok", true);
    expect(field.textContent).toBe("a");
  });
});

describe("agent-control page realm", () => {
  test("drag hands the page's own DataTransfer to its handlers", () => {
    const { window, field, api } = page(`<div id="field">Source</div><div id="to">Target</div>`);
    window.document.getElementById("to")!.setAttribute("data-longhorn-agent-ref", "to");
    let transfer: unknown;
    field.addEventListener("dragstart", (event) => {
      transfer = (event as DragEvent).dataTransfer;
    });

    expect(api.drag("field", "to")).toHaveProperty("ok", true);
    expect(transfer).toBeInstanceOf(window.DataTransfer);
  });

  test("logging an unprintable value neither throws into the page nor loses the call", () => {
    const window = openPage(`<p>Idle</p>`);
    const logged: Array<Array<unknown>> = [];
    window.console.log = (...args: Array<unknown>) => {
      logged.push(args);
    };
    const api = install(window);
    const bare = Object.create(null) as object;
    const hostile = {
      toString: () => {
        throw new Error("no text");
      },
    };

    expect(() => window.console.log("value", bare, hostile)).not.toThrow();
    expect(logged).toEqual([["value", bare, hostile]]);
    expect(api.readEvents().events.at(-1)).toEqual({
      seq: 2,
      kind: "console",
      level: "log",
      text: "value [unprintable] [unprintable]",
    });
  });
});
