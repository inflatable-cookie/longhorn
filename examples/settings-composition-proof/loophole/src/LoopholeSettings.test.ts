import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";

import { SETTINGS_APPLY_COMMAND } from "@inflatable-cookie/longhorn/settings";

import LoopholeSettings from "./LoopholeSettings.svelte";
import { SettingsProofTransport } from "./settings-proof.ts";

test("keeps advanced product authority in consumer renderers", async () => {
  const transport = new SettingsProofTransport("loophole");
  const session = transport.session();
  const reveal = vi.fn(async () => undefined);
  const probeHardware = vi.fn(async () => undefined);
  const openKeybindingEditor = vi.fn(async () => undefined);
  const mounted = render(LoopholeSettings, {
    props: {
      session,
      reveal,
      probeHardware,
      openKeybindingEditor,
    },
  });

  await mounted.findByTestId("loophole-application");
  await waitFor(() => expect(reveal).toHaveBeenCalledOnce());
  expect(
    mounted.getByRole("button", { name: "Change managed output" }),
  ).toHaveProperty("disabled", true);

  await fireEvent.click(
    mounted.getByRole("button", { name: "Change immediately" }),
  );
  await waitFor(() =>
    expect(transport.calls(SETTINGS_APPLY_COMMAND)).toBe(1),
  );

  await session.navigate({ pageId: "loophole:appearance" });
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Stage appearance" }),
  );
  expect(transport.calls(SETTINGS_APPLY_COMMAND)).toBe(1);
  await fireEvent.click(mounted.getByRole("button", { name: "Apply" }));
  await waitFor(() =>
    expect(transport.calls(SETTINGS_APPLY_COMMAND)).toBe(2),
  );

  await session.navigate({ pageId: "loophole:hardware" });
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Probe audio hardware" }),
  );
  expect(probeHardware).toHaveBeenCalledOnce();

  await session.navigate({ pageId: "loophole:keybindings" });
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Open keybinding editor" }),
  );
  expect(openKeybindingEditor).toHaveBeenCalledOnce();
  expect(transport.publications).toBe(2);

  await mounted.unmount();
  await session.stop();
});

test("does not present two staged units as one atomic apply", async () => {
  const transport = new SettingsProofTransport("loophole");
  const session = transport.session();
  const mounted = render(LoopholeSettings, {
    props: {
      session,
      reveal: async () => undefined,
      probeHardware: async () => undefined,
      openKeybindingEditor: async () => undefined,
    },
  });
  await mounted.findByTestId("loophole-application");
  await session.navigate({ pageId: "loophole:appearance" });
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Stage appearance" }),
  );
  await fireEvent.click(
    mounted.getByRole("button", { name: "Stage studio" }),
  );

  expect(
    mounted.getByText("Each dirty apply unit receives a separate receipt."),
  ).toBeTruthy();
  expect(
    mounted.getByRole("button", { name: "Apply" }),
  ).toHaveProperty("disabled", true);
  expect(transport.publications).toBe(0);

  await mounted.unmount();
  await session.stop();
});
