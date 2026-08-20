import { describe, expect, test } from "bun:test";

import { REF_ATTR, REF_PREFIX_GLOBAL } from "../../src/agent-control/index.ts";
import { findByName, install, openPage } from "./support.ts";

describe("agent-control live-DOM refs", () => {
  test("stamps refs onto elements and resolves them against the live DOM", () => {
    const window = openPage(`<button>Run</button>`);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const run = findByName(snapshot.root, "Run");
    expect(run).toBeDefined();
    const stamped = window.document.querySelector(`[${REF_ATTR}="${run!.elementRef}"]`);
    expect(stamped?.textContent).toBe("Run");
    const wait = api.waitFor({ predicate: "refResolve", element: run!.elementRef });
    expect(wait.ok && wait.holds).toBe(true);
  });

  test("a removed element yields UnresolvedRef, never a guess", () => {
    const window = openPage(`<button id="gone">Temp</button><button>Keep</button>`);
    const api = install(window);
    const first = api.snapshot();
    expect(first.ok).toBe(true);
    if (!first.ok) return;
    const temp = findByName(first.root, "Temp");
    const keep = findByName(first.root, "Keep");
    expect(temp).toBeDefined();
    window.document.getElementById("gone")?.remove();
    const click = api.click(temp!.elementRef);
    expect(click.ok).toBe(false);
    if (click.ok) return;
    expect(click.error).toEqual({ error: "unresolvedRef", element: temp!.elementRef });
    const absent = api.waitFor({ predicate: "refAbsent", element: temp!.elementRef });
    expect(absent.ok && absent.holds).toBe(true);
    expect(api.click(keep!.elementRef).ok).toBe(true);
  });

  test("two interleaved snapshots do not invalidate each other's refs", () => {
    const window = openPage(`<button id="a">Alpha</button>`);
    const api = install(window);
    const first = api.snapshot();
    expect(first.ok).toBe(true);
    if (!first.ok) return;
    const alpha = findByName(first.root, "Alpha");
    expect(alpha).toBeDefined();

    const extra = window.document.createElement("button");
    extra.textContent = "Beta";
    window.document.body.append(extra);

    const second = api.snapshot();
    expect(second.ok).toBe(true);
    if (!second.ok) return;
    const alphaAgain = findByName(second.root, "Alpha");
    const beta = findByName(second.root, "Beta");
    expect(alphaAgain?.elementRef).toBe(alpha!.elementRef);
    expect(beta?.elementRef).not.toBe(alpha!.elementRef);

    expect(api.click(alpha!.elementRef).ok).toBe(true);
    expect(api.click(beta!.elementRef).ok).toBe(true);
  });

  test("the same local seq in two webviews never cross-hits", () => {
    const ui = openPage(`<button>Ui</button>`);
    const island = openPage(`<button>Island</button>`);
    (island as unknown as Record<string, unknown>)[REF_PREFIX_GLOBAL] = encodeURIComponent("preview");
    const uiApi = install(ui);
    const islandApi = install(island);
    const uiSnap = uiApi.snapshot();
    const islandSnap = islandApi.snapshot();
    expect(uiSnap.ok && islandSnap.ok).toBe(true);
    if (!uiSnap.ok || !islandSnap.ok) return;
    const uiButton = findByName(uiSnap.root, "Ui");
    const islandButton = findByName(islandSnap.root, "Island");
    expect(uiButton?.elementRef).toMatch(/^e\d+$/);
    expect(islandButton?.elementRef).toBe(`preview:${uiButton?.elementRef}`);
    const crossIntoUi = uiApi.click(islandButton!.elementRef);
    expect(crossIntoUi.ok).toBe(false);
    if (!crossIntoUi.ok) {
      expect(crossIntoUi.error).toEqual({
        error: "unresolvedRef",
        element: islandButton!.elementRef,
      });
    }
    const crossIntoIsland = islandApi.click(uiButton!.elementRef);
    expect(crossIntoIsland.ok).toBe(false);
    if (!crossIntoIsland.ok) {
      expect(crossIntoIsland.error).toEqual({
        error: "unresolvedRef",
        element: uiButton!.elementRef,
      });
    }
    expect(uiApi.click(uiButton!.elementRef).ok).toBe(true);
    expect(islandApi.click(islandButton!.elementRef).ok).toBe(true);
  });
});
