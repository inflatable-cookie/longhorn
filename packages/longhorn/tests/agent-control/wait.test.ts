import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { findByName, install, openPage } from "./support.ts";

describe("agent-control wait_for predicates", () => {
  test("answers holds-now for the four DOM-relative predicates", () => {
    const window = openPage(`<button>Ready</button>`);
    window.document.title = "Agent Proof";
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const ready = findByName(snapshot.root, "Ready")!;
    expect(api.waitFor({ predicate: "refResolve", element: ready.elementRef })).toEqual({
      ok: true,
      holds: true,
    });
    expect(api.waitFor({ predicate: "refAbsent", element: ready.elementRef })).toEqual({
      ok: true,
      holds: false,
    });
    expect(api.waitFor({ predicate: "pageTitleContains", needle: "Proof" })).toEqual({
      ok: true,
      holds: true,
    });
    expect(api.waitFor({ predicate: "pageUrlContains", needle: "/test" })).toEqual({
      ok: true,
      holds: true,
    });
    expect(api.waitFor({ predicate: "pageUrlContains", needle: "/missing" })).toEqual({
      ok: true,
      holds: false,
    });
  });

  test("source has no timer or animation-frame wait calls", () => {
    const source = readFileSync(
      join(import.meta.dir, "../../src/agent-control/shim.ts"),
      "utf8",
    );
    expect(source).not.toContain("setTimeout(");
    expect(source).not.toContain("setInterval(");
    expect(source).not.toContain("requestAnimationFrame(");
  });
});
