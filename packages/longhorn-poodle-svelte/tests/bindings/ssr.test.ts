import { render } from "svelte/server";
import { describe, expect, it } from "vitest";

import type { StartableClientState } from "../../src/bindings/index.ts";
import LifecycleHarness from "./LifecycleHarness.svelte";

describe("SSR", () => {
  it("imports and renders without browser globals or starting the client", () => {
    let starts = 0;
    const state: StartableClientState = {
      async start() {
        starts += 1;
      },
      async stop() {},
    };

    const rendered = render(LifecycleHarness, { props: { state } });
    expect(rendered.body).toContain("mounted");
    expect(starts).toBe(0);
  });
});
