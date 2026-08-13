import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  SettingsSession,
} from "../../src/settings/poodle.ts";
import { SETTINGS_RESET_COMMAND } from "../../../longhorn/src/settings/index.ts";
import SettingsShellHarness from "./SettingsShellHarness.svelte";
import {
  FakeSettingsTransport,
  fixture,
  registry,
  requestIds,
} from "./support.ts";

function createSession(
  transport: FakeSettingsTransport,
  onClose?: () => void,
): SettingsSession {
  return new SettingsSession({
    client: transport.client(),
    nextRequestId: requestIds("shell"),
    onClose,
  });
}

/** Poodle's shell names the dialog's close from its own `closeLabel`, which
 *  defaults to this. Named once so no test has to guess it. */
const CLOSE = "Close settings";

describe("SettingsShell", () => {
  // Card 192 step 1. The dialog's own close is the only one now; the page
  // header used to carry a second, so every page offered two affordances for
  // one action. `handleDialogOpen` runs the guard, so the close is still
  // refusable -- which the dirty test below proves.
  // Card 192 step 1, and simpler than it was. The shell used to render three
  // host forms -- modal drew a Dialog with its own close, window and panel
  // drew a bare Surface with none -- so the page header carried a close for
  // the two that lacked one and duplicated it for the one that did not.
  //
  // Poodle's shell is always a dialog, so there is one close and one place it
  // lives. It stays refusable: `handleOpenChange` runs the guard, which the
  // dirty test below proves.
  it("offers exactly one close, and it is the dialog's", async () => {
    const transport = new FakeSettingsTransport();
    const mounted = render(SettingsShellHarness, {
      props: { session: createSession(transport) },
    });

    await mounted.findByTestId("consumer-page");

    expect(mounted.queryAllByRole("button", { name: CLOSE })).toHaveLength(1);
    expect(mounted.queryAllByRole("button", { name: "Close" })).toHaveLength(0);
  });

  // The section's own label, never the module's prefixed onto it. The fixture
  // registry has more than one module, which is precisely when the old rule
  // fired and produced "STORAGE · STORAGE & BACKUPS".
  it("labels a navigation group with its section alone", async () => {
    const transport = new FakeSettingsTransport();
    const mounted = render(SettingsShellHarness, {
      props: { session: createSession(transport) },
    });
    await mounted.findByTestId("consumer-page");
    const nav = document.querySelector('[aria-label="Settings pages"]');
    expect(nav?.textContent ?? "").not.toContain("·");
  });

  // Was "mounts modal, window, and panel hosts over one controller". The
  // three host forms are gone with the rewrite onto Poodle's shell, so what
  // survives of the claim is that the nav and the consumer page both arrive
  // over one controller and leave no listener behind.
  it("mounts nav and page over one controller and leaves no listener", async () => {
    const transport = new FakeSettingsTransport();
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, { props: { session } });

    await waitFor(() => {
      expect(document.querySelector('[aria-label="Settings pages"]')).not.toBeNull();
      expect(document.querySelector('[data-testid="consumer-page"]')).not.toBeNull();
    });
    await mounted.unmount();
    await session.stop();

    expect(transport.activeListenerCount()).toBe(0);
  });

  it("keeps dirty state behind the close guard", async () => {
    const transport = new FakeSettingsTransport();
    let closes = 0;
    const session = createSession(transport, () => {
      closes += 1;
    });
    const mounted = render(SettingsShellHarness, {
      props: { session },
    });
    await mounted.findByTestId("change");
    await fireEvent.click(mounted.getByTestId("change"));
    await waitFor(() => {
      expect(mounted.getByTestId("consumer-page").dataset.dirty).toBe(
        "true",
      );
    });

    await fireEvent.click(mounted.getByRole("button", { name: CLOSE }));
    expect(
      await mounted.findByRole("alertdialog", {
        name: "Unsaved changes",
      }),
    ).toBeTruthy();
    await fireEvent.click(mounted.getByRole("button", { name: "Stay" }));
    expect(closes).toBe(0);
    expect(session.dirty).toBe(true);

    await fireEvent.click(mounted.getByRole("button", { name: CLOSE }));
    await fireEvent.click(
      mounted.getByRole("button", { name: "Discard" }),
    );
    await waitFor(() => expect(closes).toBe(1));
    expect(session.dirty).toBe(false);
    await mounted.unmount();
    await session.stop();
  });

  it("resolves search anchors and focuses the structural target", async () => {
    const transport = new FakeSettingsTransport();
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, {
      props: { session },
    });
    const search = await mounted.findByRole("searchbox", {
      name: "Search settings",
    });
    await fireEvent.input(search, { target: { value: "Output" } });

    // Search narrows the rail rather than drawing a second list over the
    // page. The anchor is its own entry, so picking it lands on the control
    // rather than on the page that contains it.
    await waitFor(() => {
      const nav = document.querySelector('[aria-label="Settings pages"]');
      expect(nav?.textContent ?? "").toContain("Audio · Output device");
    });

    const result = await mounted.findByText("Audio · Output device");
    await fireEvent.click(result);

    await waitFor(() => {
      const target = document.querySelector<HTMLElement>(
        '[data-page-id="app:audio"][data-anchor-id="app:output"]',
      );
      expect(target).not.toBeNull();
      expect(document.activeElement).toBe(target);
    });

    // The query survives. Clearing it on the first pick would throw away the
    // filter at the moment it became useful.
    expect((search as HTMLInputElement).value).toBe("Output");
    await mounted.unmount();
    await session.stop();
  });

  it("preserves the draft and announces a conflict", async () => {
    const transport = new FakeSettingsTransport();
    transport.mutationValue = structuredClone(
      fixture.mutationResults[2],
    ) as typeof transport.mutationValue;
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, {
      props: { session },
    });
    await fireEvent.click(await mounted.findByTestId("change"));
    await fireEvent.click(
      mounted.getByRole("button", { name: "Apply" }),
    );

    expect(
      await mounted.findByText("Settings changed elsewhere"),
    ).toBeTruthy();
    expect(mounted.getByTestId("consumer-page").dataset.dirty).toBe(
      "true",
    );
    await mounted.unmount();
    await session.stop();
  });

  it("never renders Saved after an immediate failure", async () => {
    const transport = new FakeSettingsTransport();
    transport.registryValue = registry("immediate");
    transport.mutationError = new Error("offline");
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, {
      props: { session },
    });
    await fireEvent.click(await mounted.findByTestId("change"));

    expect(await mounted.findByText("Save failed")).toBeTruthy();
    expect(mounted.queryByText("Saved")).toBeNull();
    expect(mounted.queryByRole("button", { name: "Apply" })).toBeNull();
    expect(mounted.queryByRole("button", { name: "Cancel" })).toBeNull();
    await mounted.unmount();
    await session.stop();
  });

  it("uses a confirmation surface for scoped reset", async () => {
    const transport = new FakeSettingsTransport();
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, {
      props: { session },
    });
    await fireEvent.click(await mounted.findByTestId("reset"));
    expect(
      await mounted.findByRole("alertdialog", { name: "Reset settings" }),
    ).toBeTruthy();
    await fireEvent.click(
      mounted.getByRole("button", { name: "Reset" }),
    );
    await waitFor(() => {
      expect(transport.calls(SETTINGS_RESET_COMMAND)).toBe(1);
    });
    await mounted.unmount();
    await session.stop();
  });

  it("shows missing renderer failure before consumer reveal", async () => {
    const transport = new FakeSettingsTransport();
    const session = createSession(transport);
    const mounted = render(SettingsShellHarness, {
      props: {
        session,
        missingRenderer: true,
      },
    });

    expect(await mounted.findByText("Settings failed")).toBeTruthy();
    expect(mounted.queryByTestId("consumer-page")).toBeNull();
    await mounted.unmount();
    await session.stop();
  });

  it("remounts one session without leaking listeners or pending state", async () => {
    const transport = new FakeSettingsTransport();
    const session = createSession(transport);

    for (let mount = 0; mount < 2; mount += 1) {
      const mounted = render(SettingsShellHarness, {
        props: { session },
      });
      await mounted.findByTestId("consumer-page");
      await mounted.unmount();
      await session.stop();
      expect(transport.activeListenerCount()).toBe(0);
      expect(session.status.kind).toBe("idle");
    }
    expect(transport.unlistenCount).toBe(4);
  });
});
