import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { LayoutState } from "@inflatable-cookie/longhorn-svelte/layout";

import NucleusShell from "./NucleusShell.svelte";
import { document as layoutDocument } from "./model.ts";

function layoutState(dispatch = true): LayoutState {
  return new LayoutState(
    dispatch
      ? {
          dispatch: async () =>
            new Promise(() => {
              // No mutation in the shell proof.
            }),
        }
      : { unsupportedReason: "registered layout commands are absent" },
  );
}

describe("Nucleus shell", () => {
  it("loads authority before reveal and mounts five Surface-free regions", async () => {
    const events: string[] = [];
    const screen = render(NucleusShell, {
      props: {
        layoutState: layoutState(),
        async loadAuthority() {
          events.push("authority");
          return layoutDocument;
        },
        async reveal() {
          events.push("reveal");
        },
      },
    });

    await waitFor(() =>
      expect(screen.getByRole("main", { name: "Nucleus workspace" })).toBeTruthy(),
    );
    await waitFor(() => expect(events).toEqual(["authority", "reveal"]));
    expect(screen.getAllByRole("region")).toHaveLength(5);
    expect(screen.getByRole("tab", { name: "Project" })).toBeTruthy();
    expect(globalThis.document.documentElement.dataset.theme).toBe("graphite");
    await screen.unmount();
  });

  it("shows missing capability and host-load failures", async () => {
    const unsupported = render(NucleusShell, {
      props: {
        layoutState: layoutState(false),
        loadAuthority: async () => layoutDocument,
        reveal: async () => undefined,
      },
    });
    await waitFor(() =>
      expect(unsupported.getByText("registered layout commands are absent")).toBeTruthy(),
    );
    expect(unsupported.queryByRole("main")).toBeNull();
    await unsupported.unmount();

    const failed = render(NucleusShell, {
      props: {
        layoutState: layoutState(),
        loadAuthority: async () => {
          throw new Error("layout snapshot command failed");
        },
        reveal: async () => undefined,
      },
    });
    await waitFor(() =>
      expect(failed.getByText("layout snapshot command failed")).toBeTruthy(),
    );
    expect(failed.queryByRole("main")).toBeNull();
    await failed.unmount();
  });
});
