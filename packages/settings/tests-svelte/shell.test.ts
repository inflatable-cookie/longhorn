import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  SettingsSession,
} from "../src/poodle.ts";
import { SETTINGS_RESET_COMMAND } from "../src/index.ts";
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

describe("SettingsShell", () => {
  it("mounts modal, window, and panel hosts over one controller", async () => {
    for (const host of ["modal", "window", "panel"] as const) {
      const transport = new FakeSettingsTransport();
      const session = createSession(transport);
      const mounted = render(SettingsShellHarness, {
        props: { session, host },
      });

      await waitFor(() => {
        expect(
          document.querySelector(`[data-host="${host}"]`),
        ).not.toBeNull();
        expect(
          document.querySelector('[aria-label="Settings pages"]'),
        ).not.toBeNull();
        expect(
          document.querySelector('[data-testid="consumer-page"]'),
        ).not.toBeNull();
      });
      await mounted.unmount();
      await session.stop();
      expect(transport.activeListenerCount()).toBe(0);
    }
  });

  it("keeps dirty state behind the close guard", async () => {
    const transport = new FakeSettingsTransport();
    let closes = 0;
    const session = createSession(transport, () => {
      closes += 1;
    });
    const mounted = render(SettingsShellHarness, {
      props: { session, host: "window" },
    });
    await mounted.findByTestId("change");
    await fireEvent.click(mounted.getByTestId("change"));
    await waitFor(() => {
      expect(mounted.getByTestId("consumer-page").dataset.dirty).toBe(
        "true",
      );
    });

    await fireEvent.click(mounted.getByRole("button", { name: "Close" }));
    expect(
      await mounted.findByRole("alertdialog", {
        name: "Unsaved changes",
      }),
    ).toBeTruthy();
    await fireEvent.click(mounted.getByRole("button", { name: "Stay" }));
    expect(closes).toBe(0);
    expect(session.dirty).toBe(true);

    await fireEvent.click(mounted.getByRole("button", { name: "Close" }));
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
      props: { session, host: "panel" },
    });
    const search = await mounted.findByRole("searchbox", {
      name: "Search settings",
    });
    await fireEvent.input(search, { target: { value: "Output" } });
    const result = await mounted.findByRole("button", {
      name: "Audio · Output device",
    });
    await fireEvent.click(result);

    await waitFor(() => {
      const target = document.querySelector<HTMLElement>(
        '[data-page-id="app:audio"][data-anchor-id="app:output"]',
      );
      expect(target).not.toBeNull();
      expect(document.activeElement).toBe(target);
    });
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
      props: { session, host: "window" },
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
      props: { session, host: "window" },
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
      props: { session, host: "window" },
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
        host: "window",
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
        props: { session, host: "panel" },
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
