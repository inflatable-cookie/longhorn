import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import CommandSessionHarness from "./CommandSessionHarness.svelte";
import KeybindingHarness from "./KeybindingHarness.svelte";
import { createSession } from "./support.ts";

describe("command Svelte and public Poodle adapters", () => {
  it("keeps mounted sessions independent and tears down listeners", async () => {
    const first = createSession();
    const second = createSession();
    const firstMount = render(CommandSessionHarness, {
      props: { session: first.session },
    });
    const secondMount = render(CommandSessionHarness, {
      props: { session: second.session },
    });

    await waitFor(() => {
      expect(first.session.status.kind).toBe("ready");
      expect(second.session.status.kind).toBe("ready");
    });
    expect(first.target.listeners.size).toBe(1);
    expect(second.target.listeners.size).toBe(1);

    await fireEvent.click(
      firstMount.container.querySelector("button")!,
    );
    expect(
      await firstMount.findByRole("dialog", { name: "Command palette" }),
    ).toBeTruthy();
    expect(second.session.open).toBe(false);

    await firstMount.unmount();
    await first.session.stop();
    expect(first.target.listeners.size).toBe(0);
    expect(second.target.listeners.size).toBe(1);
    await secondMount.unmount();
    await second.session.stop();
    expect(second.target.listeners.size).toBe(0);
  });

  it("capture consumes one chord without dispatch and clears on teardown", async () => {
    const state = createSession();
    await state.session.start();
    state.session.beginCapture("base:open");
    let consumed = 0;
    state.target.dispatch({
      code: "KeyK",
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: true,
      repeat: false,
      preventDefault: () => {
        consumed += 1;
      },
      stopPropagation: () => {
        consumed += 1;
      },
    });
    expect(consumed).toBe(2);
    expect(state.session.captured?.label).toBe("⌘K");
    expect(state.executions).toHaveLength(0);
    await state.session.stop();
    expect(state.session.captured).toBeUndefined();
    expect(state.target.listeners.size).toBe(0);
  });

  it("binds keybinding records through public controlled callbacks", async () => {
    const state = createSession();
    await state.session.start();
    let captured = "";
    let applied = 0;
    const mounted = render(KeybindingHarness, {
      props: {
        records: state.session.settingsRecords,
        onCapture: (bindingId: string) => {
          captured = bindingId;
        },
        onApply: () => {
          applied += 1;
        },
      },
    });
    await fireEvent.click(
      mounted.getByRole("button", { name: "Change base:open" }),
    );
    await fireEvent.click(mounted.getByRole("button", { name: "Apply" }));
    expect(captured).toBe("base:open");
    expect(applied).toBe(1);
    await mounted.unmount();
    await state.session.stop();
  });
});
