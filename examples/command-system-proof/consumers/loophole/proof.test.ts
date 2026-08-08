import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { SETTINGS_PROTOCOL_VERSION } from "@inflatable-cookie/longhorn/settings";
import {
  CommandSession,
} from "@inflatable-cookie/longhorn-poodle-svelte/commands/svelte";

import {
  KeyboardTarget,
  contextPath,
  createHarness,
} from "../../common.ts";
import RichCommandShell from "./RichCommandShell.svelte";

describe("Loophole rich command composition", () => {
  it("mounts public Poodle palette and settings over one session", async () => {
    expect(SETTINGS_PROTOCOL_VERSION).toBe(1);
    const state = createHarness("loophole");
    const keyboard = new KeyboardTarget();
    const session = new CommandSession({
      controller: state.controller,
      platform: "macOs",
      contextPath: () => contextPath("loophole"),
      keyboardTarget: keyboard,
    });
    await session.start();
    const mounted = render(RichCommandShell, { props: { session } });
    await waitFor(() => expect(session.status.kind).toBe("ready"));
    await fireEvent.click(
      mounted.getByRole("button", { name: "Open commands" }),
    );
    expect(
      await mounted.findByRole("dialog", { name: "Loophole commands" }),
    ).toBeTruthy();
    expect(
      mounted.container.querySelector(
        '[data-command-id="loophole:transport.play"]',
      ),
    ).toBeTruthy();
    await mounted.unmount();
    await session.stop();
    expect(keyboard.listeners.size).toBe(0);
    expect(state.catalogue.listeners.size).toBe(0);
  });
});
