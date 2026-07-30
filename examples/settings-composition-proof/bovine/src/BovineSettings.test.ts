import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";

import {
  SETTINGS_APPLY_COMMAND,
  SETTINGS_RESET_COMMAND,
} from "@longhorn/settings";

import BovineSettings from "./BovineSettings.svelte";
import { SettingsProofTransport } from "./settings-proof.ts";

test("mounts one staged modal after authoritative load", async () => {
  const transport = new SettingsProofTransport("bovine");
  const session = transport.session();
  const reveal = vi.fn(async () => undefined);
  const mounted = render(BovineSettings, { props: { session, reveal } });

  await mounted.findByTestId("bovine-preferences");
  await waitFor(() => expect(reveal).toHaveBeenCalledOnce());
  expect(transport.authorityLoadedBeforeMutation()).toBe(true);

  await fireEvent.click(
    mounted.getByRole("button", { name: "Stage compact editor" }),
  );
  expect(transport.calls(SETTINGS_APPLY_COMMAND)).toBe(0);
  await fireEvent.click(mounted.getByRole("button", { name: "Apply" }));
  await waitFor(() =>
    expect(transport.calls(SETTINGS_APPLY_COMMAND)).toBe(1),
  );
  expect(transport.publications).toBe(1);

  await fireEvent.click(
    mounted.getByRole("button", { name: "Reset preference" }),
  );
  await waitFor(() =>
    expect(transport.calls(SETTINGS_RESET_COMMAND)).toBe(1),
  );

  await mounted.unmount();
  await session.stop();
  expect(transport.activeListenerCount()).toBe(0);
});

test.each([
  ["conflict", "Settings changed elsewhere"],
  ["invalidIntent", "Change rejected"],
  ["policyBlocked", "Change rejected"],
  ["recoveryRequired", "Recovery required"],
] as const)(
  "publishes nothing for %s authority",
  async (mode, message) => {
    const transport = new SettingsProofTransport("bovine");
    if (mode === "recoveryRequired") {
      transport.setRecoveryRequired();
    } else {
      transport.mutationMode = mode;
    }
    const session = transport.session();
    const mounted = render(BovineSettings, {
      props: { session, reveal: async () => undefined },
    });
    await fireEvent.click(
      await mounted.findByRole("button", { name: "Stage compact editor" }),
    );
    await fireEvent.click(mounted.getByRole("button", { name: "Apply" }));

    expect(await mounted.findByText(message)).toBeTruthy();
    expect(transport.publications).toBe(0);
    await mounted.unmount();
    await session.stop();
  },
);
