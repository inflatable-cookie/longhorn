import { fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import HistoryPanelHarness from "./HistoryPanelHarness.svelte";
import { createMountedSession } from "./support.ts";

describe("history Svelte and public Poodle adapters", () => {
  it("keeps mounted sessions independent and tears down listeners", async () => {
    const first = createMountedSession();
    const second = createMountedSession();
    const firstMount = render(HistoryPanelHarness, {
      props: { session: first.session },
    });
    const secondMount = render(HistoryPanelHarness, {
      props: { session: second.session },
    });

    await waitFor(() => {
      expect(first.session.status.kind).toBe("ready");
      expect(second.session.status.kind).toBe("ready");
    });
    expect(first.port.listeners.size).toBe(1);
    expect(second.port.listeners.size).toBe(1);

    await fireEvent.click(
      within(firstMount.container).getByRole("button", {
        name: "Undo Move clip",
      }),
    );
    await waitFor(() => {
      expect(first.session.snapshot?.summary.revision).toBe(12);
    });
    expect(second.session.snapshot?.summary.revision).toBe(11);

    await firstMount.unmount();
    await waitFor(() => expect(first.port.listeners.size).toBe(0));
    expect(first.port.unlistenCount).toBe(1);
    expect(second.port.listeners.size).toBe(1);

    await secondMount.unmount();
    await waitFor(() => expect(second.port.listeners.size).toBe(0));
  });

  it("binds filtering, checkout, loading, and pagination through public controls", async () => {
    const state = createMountedSession();
    const mounted = render(HistoryPanelHarness, {
      props: { session: state.session },
    });
    await waitFor(() => expect(state.session.status.kind).toBe("ready"));

    const filter = mounted.getByRole("searchbox", { name: "Filter history" });
    await fireEvent.input(filter, { target: { value: "rename" } });
    await waitFor(() =>
      expect(state.session.entries.map(({ entryId }) => entryId)).toEqual([
        "entry:future",
      ]),
    );
    expect(mounted.getByRole("button", { name: "Rename track" })).toBeTruthy();

    await fireEvent.click(mounted.getByRole("button", { name: "Rename track" }));
    await waitFor(() =>
      expect(state.session.snapshot?.summary.revision).toBe(12),
    );

    await mounted.unmount();
  });
});
