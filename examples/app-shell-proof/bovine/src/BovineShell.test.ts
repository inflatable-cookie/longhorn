import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  ReactiveClientState,
  type ReactiveConnection,
} from "@inflatable-cookie/longhorn-poodle-svelte";

import BovineShell, {
  type BovineAuthority,
} from "./BovineShell.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

describe("Bovine shell", () => {
  it("reveals only after authority and tears down its connection", async () => {
    const ready = deferred<void>();
    const events: string[] = [];
    let current: BovineAuthority | undefined;
    let disposals = 0;
    const state = new ReactiveClientState<BovineAuthority>({
      capability: {
        kind: "supported",
        connect: () =>
          ({
            ready: ready.promise,
            current: () => current,
            async dispose() {
              disposals += 1;
            },
          }) satisfies ReactiveConnection<BovineAuthority>,
      },
    });
    const screen = render(BovineShell, {
      props: {
        clientState: state,
        async reveal() {
          events.push("reveal");
        },
      },
    });

    expect(screen.getByText("Loading workspace authority")).toBeTruthy();
    expect(events).toEqual([]);
    current = {
      documentTitle: "Bovine Accelerator",
      sectionTitle: "Accounts",
    };
    events.push("authority");
    ready.resolve();

    await waitFor(() =>
      expect(screen.getByRole("main", { name: "Bovine workspace" })).toBeTruthy(),
    );
    expect(events).toEqual(["authority", "reveal"]);
    expect(document.documentElement.dataset.theme).toBe("clay");
    expect(screen.getByText("Product-owned content stays outside Longhorn.")).toBeTruthy();

    await screen.unmount();
    await waitFor(() => expect(disposals).toBe(1));
  });

  it("renders an unsupported host instead of a fallback document", async () => {
    const state = new ReactiveClientState<BovineAuthority>({
      capability: {
        kind: "unsupported",
        reason: "guarded reveal command is absent",
      },
    });
    const screen = render(BovineShell, {
      props: { clientState: state, reveal: async () => undefined },
    });

    await waitFor(() =>
      expect(screen.getByText("guarded reveal command is absent")).toBeTruthy(),
    );
    expect(screen.queryByRole("main")).toBeNull();
  });
});
