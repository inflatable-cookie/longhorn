import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import App from "./App.svelte";

afterEach(cleanup);

describe("greenfield shell lifecycle", () => {
  test("keeps loading, failure, authority, and teardown explicit", async () => {
    const teardown = vi.fn();
    const mounted = render(App, {
      shape: "proof",
      selectedModules: ["@longhorn/core"],
      status: { kind: "loading" },
      onTeardown: teardown,
    });
    expect(screen.getByText("Loading authoritative desktop state")).toBeTruthy();

    await mounted.rerender({
      shape: "proof",
      selectedModules: ["@longhorn/core"],
      status: { kind: "failed", detail: "capability missing" },
      onTeardown: teardown,
    });
    expect(screen.getByText("capability missing")).toBeTruthy();

    await mounted.rerender({
      shape: "proof",
      selectedModules: ["@longhorn/core"],
      status: { kind: "ready", authority: "revision:1" },
      onTeardown: teardown,
    });
    expect(document.querySelector("[data-authority='revision:1']")).toBeTruthy();
    mounted.unmount();
    expect(teardown).toHaveBeenCalledTimes(1);
  });
});
