import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { HistorySession } from "@longhorn/history/svelte";

import { createControllerHarness, rendererFixture } from "../../common.ts";
import fixtureJson from "./fixture.json";
import RichHistoryPanel from "./RichHistoryPanel.svelte";

describe("Loophole rich history composition", () => {
  it("mounts the public Poodle panel and tears down its listener", async () => {
    const fixture = rendererFixture(fixtureJson);
    const harness = createControllerHarness(fixture);
    const session = new HistorySession({ port: harness.port, pageSize: 50 });
    const mounted = render(RichHistoryPanel, { props: { session } });
    await waitFor(() => expect(session.status.kind).toBe("ready"));
    expect(
      mounted.getByRole("button", {
        name: fixture.initialSnapshot.summary.nextUndoLabel
          ? `Undo ${fixture.initialSnapshot.summary.nextUndoLabel}`
          : "Undo",
      }),
    ).toBeTruthy();
    expect(
      mounted.getByRole("button", {
        name: fixture.initialSnapshot.summary.nextRedoLabel
          ? `Redo ${fixture.initialSnapshot.summary.nextRedoLabel}`
          : "Redo",
      }),
    ).toBeTruthy();
    await fireEvent.input(mounted.getByRole("searchbox"), {
      target: { value: "rename" },
    });
    expect(session.filter).toBe("rename");
    await mounted.unmount();
    await session.dispose();
    expect(harness.counters.unlisten).toBe(1);
  });
});
