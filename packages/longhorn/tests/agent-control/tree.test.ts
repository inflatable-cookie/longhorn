import { describe, expect, test } from "bun:test";

import {
  MAX_NODES,
  TRUNCATED_REF,
  TRUNCATED_ROLE,
} from "../../src/agent-control/index.ts";
import { collectRoles, findByName, findByRole, install, openPage } from "./support.ts";

describe("agent-control semantic tree", () => {
  test("renders roles, names, values, and interaction state", () => {
    const window = openPage(`
      <h1>Inbox</h1>
      <form>
        <label for="name">Name</label>
        <input id="name" value="Ada" />
        <label><input type="checkbox" checked /> Subscribe</label>
        <button type="submit" disabled>Save</button>
      </form>
    `);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    expect(snapshot.page.title).toBe("Proof");
    expect(snapshot.page.url).toContain("/test");

    const heading = findByRole(snapshot.root, "heading");
    expect(heading?.name).toBe("Inbox");
    expect(heading?.states).toContain("visible");

    const textbox = findByName(snapshot.root, "Name");
    expect(textbox?.role).toBe("textbox");
    expect(textbox?.value).toBe("Ada");

    const checkbox = findByRole(snapshot.root, "checkbox");
    expect(checkbox?.states).toContain("checked");

    const save = findByName(snapshot.root, "Save");
    expect(save?.role).toBe("button");
    expect(save?.states).toContain("disabled");
    expect(collectRoles(snapshot.root)).toContain("form");
  });

  test("marks truncation explicitly rather than silently capping", () => {
    const items = Array.from({ length: MAX_NODES + 8 }, (_, index) => `<button>B${index}</button>`).join("");
    const window = openPage(`<div>${items}</div>`);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const roles = collectRoles(snapshot.root);
    expect(roles).toContain(TRUNCATED_ROLE);
    const truncated = findByRole(snapshot.root, TRUNCATED_ROLE);
    expect(truncated?.elementRef).toBe(TRUNCATED_REF);
  });

  test("skips hidden nodes and script/style", () => {
    const window = openPage(`
      <button>Visible</button>
      <button hidden>Hidden</button>
      <div aria-hidden="true"><button>Aria hidden</button></div>
      <script>window.__planted = true</script>
      <style>.x { color: red }</style>
    `);
    const api = install(window);
    const snapshot = api.snapshot();
    expect(snapshot.ok).toBe(true);
    if (!snapshot.ok) return;
    const names: string[] = [];
    const walk = (node: typeof snapshot.root) => {
      if (node.name) names.push(node.name);
      node.children.forEach(walk);
    };
    walk(snapshot.root);
    expect(names).toContain("Visible");
    expect(names).not.toContain("Hidden");
    expect(names).not.toContain("Aria hidden");
    expect(collectRoles(snapshot.root)).not.toContain("script");
  });
});
