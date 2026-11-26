import { fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { CommandKeymapConflict } from "@inflatable-cookie/longhorn/commands/protocol";
import { KeybindingSettings } from "../../src/commands/poodle.ts";
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

    await fireEvent.click(firstMount.container.querySelector("button")!);
    expect(await firstMount.findByRole("dialog", { name: "Command palette" })).toBeTruthy();
    expect(second.session.open).toBe(false);

    await Promise.resolve(firstMount.unmount());
    await first.session.stop();
    expect(first.target.listeners.size).toBe(0);
    expect(second.target.listeners.size).toBe(1);
    await Promise.resolve(secondMount.unmount());
    await second.session.stop();
    expect(second.target.listeners.size).toBe(0);
  });

  it("shows search errors in the open palette and recovers on a new query", async () => {
    const { session } = createSession({
      search: async (_records, query) => {
        if (query === "broken") {
          throw new Error("search service unavailable");
        }
        return [];
      },
    });
    const mounted = render(CommandSessionHarness, { props: { session } });
    await waitFor(() => expect(session.status.kind).toBe("ready"));
    await fireEvent.click(mounted.getByRole("button", { name: "Open commands" }));
    const dialog = within(await mounted.findByRole("dialog", { name: "Command palette" }));
    const search = dialog.getByRole("searchbox", { name: "Search commands" });
    await fireEvent.input(search, { target: { value: "broken" } });
    const message = "Could not search commands. Change the search and try again.";
    expect(await dialog.findByText(message)).toBeTruthy();

    await fireEvent.input(search, { target: { value: "working" } });
    await waitFor(() => expect(dialog.queryByText(message)).toBeNull());
    mounted.unmount();
    await session.stop();
  });

  it.each([
    [
      "typed rejection",
      async () => ({ status: "rejected" }) as const,
      "The app refused to run this command.",
    ],
    [
      "transport failure",
      async () => {
        throw new Error("command service unavailable");
      },
      "The command failed. Try again.",
    ],
  ] as const)("makes a selection %s visible and allows retry", async (_label, execute, message) => {
    const { session, executions } = createSession({ execute });
    const mounted = render(CommandSessionHarness, { props: { session } });
    await waitFor(() => expect(session.status.kind).toBe("ready"));
    await fireEvent.click(mounted.getByRole("button", { name: "Open commands" }));
    const dialog = within(await mounted.findByRole("dialog", { name: "Command palette" }));

    const command = await dialog.findByRole("button", { name: "app:open" });
    await fireEvent.click(command);
    expect(await dialog.findByText(message)).toBeTruthy();
    await fireEvent.click(command);
    await waitFor(() => expect(executions).toHaveLength(2));
    mounted.unmount();
    await session.stop();
  });

  it("finds keybindings by mixed-case command id using trimmed search terms", async () => {
    const { session } = createSession();
    await session.start();
    const records = session.settingsRecords.map((record) =>
      record.id === "app:open" ? { ...record, id: "app:InspectFile", label: "Open file" } : record,
    );
    const mounted = render(KeybindingSettings, { records, query: "  inspectFILE  " });
    expect(mounted.getAllByRole("listitem")).toHaveLength(1);
    expect(mounted.getByText("Open file")).toBeTruthy();
    await session.stop();
  });

  it("resets the keymap only after confirmation", async () => {
    let resets = 0;
    const mounted = render(KeybindingSettings, {
      records: [],
      onReset: () => {
        resets += 1;
      },
    });
    const reset = mounted.getByRole("button", { name: "Reset" });
    await fireEvent.click(reset);
    const dialog = within(
      await mounted.findByRole("alertdialog", { name: "Reset keyboard shortcuts?" }),
    );
    expect(resets).toBe(0);
    await fireEvent.click(dialog.getByRole("button", { name: "Cancel" }));
    expect(resets).toBe(0);

    await fireEvent.click(reset);
    const confirmation = within(
      await mounted.findByRole("alertdialog", { name: "Reset keyboard shortcuts?" }),
    );
    await fireEvent.click(confirmation.getByRole("button", { name: "Reset shortcuts" }));
    expect(resets).toBe(1);
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
    const openRecord = state.session.settingsRecords.find(({ id }) => id === "app:open");
    const shortcut = openRecord?.shortcuts[0];
    if (shortcut === undefined) {
      throw new Error("missing open shortcut");
    }
    const conflict: CommandKeymapConflict = {
      platform: "macOs",
      chord: shortcut.chord,
      contextId: shortcut.contextId,
      bindingIds: [shortcut.bindingId, "custom:open"],
      invocations: [
        { commandId: "app:open", arguments: {} },
        { commandId: "app:close", arguments: {} },
      ],
    };
    let captured = "";
    let applied = 0;
    const mounted = render(KeybindingHarness, {
      props: {
        records: state.session.settingsRecords.map((record) =>
          record === openRecord ? { ...record, label: "Open file" } : record,
        ),
        conflicts: [conflict],
        onCapture: (bindingId: string) => {
          captured = bindingId;
        },
        onApply: () => {
          applied += 1;
        },
      },
    });
    expect(mounted.getByText("1 unresolved keybinding conflict").textContent?.trim()).toBe(
      "1 unresolved keybinding conflict",
    );
    const open = mounted.getByRole("button", { name: "Change ⌘O for Open file" });
    expect(open.textContent?.trim()).toBe("Change ⌘O");
    expect(mounted.queryByText("Change base:open")).toBeNull();
    await fireEvent.click(open);
    await fireEvent.click(mounted.getByRole("button", { name: "Apply" }));
    expect(captured).toBe("base:open");
    expect(applied).toBe(1);
    await Promise.resolve(mounted.unmount());
    await state.session.stop();
  });
});
