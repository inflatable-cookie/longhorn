// Shim output against the samples Serde writes (`effigy generate:bindings`).
import assert from "node:assert/strict";
import { describe, expect, test } from "bun:test";

import protocol from "../../../../fixtures/agent-control/protocol-v1.json";
import type { SemanticNode, WaitPredicate } from "../../src/agent-control/index.ts";
import { install, openPage } from "./support.ts";

// oxlint-disable-next-line quality/no-object-parameters -- Compares the key set of any wire object.
const keys = (value: object): Set<string> => new Set(Object.keys(value));

function every(node: SemanticNode): Array<SemanticNode> {
  return [node, ...node.children.flatMap(every)];
}

describe("agent-control wire shapes", () => {
  test("every snapshot node carries the Rust node's fields, absent ones as null", () => {
    const api = install(openPage(`<div><button>Save</button><input /></div>`));
    const snapshot = api.snapshot();
    assert(snapshot.ok);

    expect(keys(snapshot.page)).toEqual(keys(protocol.page));
    const nodes = every(snapshot.root);
    for (const node of nodes) {
      expect(keys(node)).toEqual(keys(protocol.snapshotRoot));
    }
    expect(nodes.find((node) => node.role === "textbox")).toMatchObject({
      name: null,
      value: null,
    });
  });

  test("an event batch carries the Rust batch's fields", () => {
    const window = openPage(`<p>Idle</p>`);
    const api = install(window);
    window.console.log("ready");
    window.history.pushState(null, "", "/next");
    window.dispatchEvent(new window.ErrorEvent("error", { message: "boom" }));
    const batch = api.readEvents();

    expect(keys(batch)).toEqual(keys(protocol.pageEvents));
    for (const event of batch.events) {
      const sample = protocol.pageEvents.events.find((candidate) => candidate.kind === event.kind);
      assert(sample);
      expect(keys(event)).toEqual(keys(sample));
    }
    expect(new Set(batch.events.map((event) => event.kind))).toEqual(
      new Set(["navigation", "console", "error"]),
    );
  });

  test("each Rust wait predicate is admitted", () => {
    const api = install(openPage(`<p>Idle</p>`));
    for (const predicate of protocol.waitPredicates) {
      expect(api.waitFor(predicate as WaitPredicate).ok).toBe(true);
    }
  });

  test("files as Rust serializes them are assigned, with or without a media type", () => {
    const window = openPage(`<input id="field" type="file" multiple />`);
    window.document.getElementById("field")!.setAttribute("data-longhorn-agent-ref", "field");
    const api = install(window);

    expect(protocol.fileInputFiles[1]).toMatchObject({ mediaType: null });
    expect(api.setFileInput("field", protocol.fileInputFiles)).toHaveProperty("ok", true);
  });

  test("the shim reports its failures in the Rust error shapes", () => {
    const window = openPage(`<button id="field">Go</button>`);
    window.document.getElementById("field")!.setAttribute("data-longhorn-agent-ref", "field");
    const api = install(window);

    const [unresolved, unsupported] = protocol.shimErrors;
    expect(api.click("e2")).toHaveProperty("error", unresolved);
    expect(api.setFileInput("field", protocol.fileInputFiles)).toHaveProperty("error", unsupported);
  });
});
