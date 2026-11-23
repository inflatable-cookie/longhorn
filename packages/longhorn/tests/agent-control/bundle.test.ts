import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { EVENT_BUFFER_LIMIT, SHIM_GLOBAL } from "../../src/agent-control/index.ts";
import { install, openPage } from "./support.ts";

describe("agent-control packaging", () => {
  test("bundles standalone as an IIFE with no imports", async () => {
    const entry = join(import.meta.dir, "../../src/agent-control/inject.ts");
    const result = await Bun.build({
      entrypoints: [entry],
      format: "iife",
      target: "browser",
      minify: false,
    });
    expect(result.success).toBe(true);
    const joined = (await Promise.all(result.outputs.map((artifact) => artifact.text()))).join("");
    expect(joined.length).toBeGreaterThan(0);
    expect(joined).toContain(SHIM_GLOBAL);
    expect(joined).toContain("data-longhorn-agent-ref");
    expect(joined).not.toMatch(/\bfrom\s+["']/);
    expect(joined).not.toMatch(/\bimport\s+/);
    const committed = readFileSync(
      join(
        import.meta.dir,
        "../../../../crates/longhorn-tauri-agent-control/src/agent_control_shim.js",
      ),
      "utf8",
    );
    expect(committed).toBe(joined);
  });

  test("install is idempotent and event reads do not cross cursors", () => {
    const window = openPage(`<button>Hi</button>`);
    const first = install(window);
    const second = install(window);
    expect(second).toBe(first);
    window.console.log("one");
    const early = first.readEvents(0);
    expect(early.epoch.length).toBeGreaterThan(0);
    expect(early.events.some((event) => event.kind === "console" && event.text === "one")).toBe(
      true,
    );
    window.console.log("two");
    const later = first.readEvents(early.nextSeq - 1, early.epoch);
    expect(later.epoch).toBe(early.epoch);
    expect(later.events.map((event) => ("text" in event ? event.text : ""))).toEqual(["two"]);
    expect(early.dropped).toBe(0);
    expect(EVENT_BUFFER_LIMIT).toBeGreaterThan(0);

    // A reloaded document gets a fresh epoch, records its full-document load
    // as a navigation event at seq 1, and passing the old cursor + old epoch
    // resets to seq 0 so neither the navigation nor early events are lost.
    const reloaded = openPage(`<button>Reloaded</button>`);
    const reloadedApi = install(reloaded);
    reloaded.console.log("before-load-finish");
    const afterReload = reloadedApi.readEvents(later.nextSeq - 1, later.epoch);
    expect(afterReload.epoch).not.toBe(early.epoch);
    expect(afterReload.events[0]?.kind).toBe("navigation");
    expect(afterReload.events[0]?.seq).toBe(1);
    expect(
      afterReload.events
        .filter((event) => event.kind === "console")
        .map((event) => ("text" in event ? event.text : "")),
    ).toEqual(["before-load-finish"]);
  });
});
