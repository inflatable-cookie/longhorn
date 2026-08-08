import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";

import {
  CONFIG_RESTORE_EXECUTE_COMMAND,
  CONFIG_RESTORE_INSPECT_COMMAND,
  CONFIG_RESTORE_PLAN_COMMAND,
  type ConfigOperationsSnapshot,
} from "@inflatable-cookie/longhorn-config";
import { SETTINGS_APPLY_COMMAND } from "@inflatable-cookie/longhorn-settings";

import SoundcheckSettings from "./SoundcheckSettings.svelte";
import configFixture from "./fixtures/config-protocol-v1.json";
import { ConfigProofTransport } from "./config-proof.ts";
import { SettingsProofTransport } from "./settings-proof.ts";

test("composes product, storage, backup, and restore pages in one window", async () => {
  const settings = new SettingsProofTransport("soundcheck");
  const config = new ConfigProofTransport();
  const session = settings.session();
  const reveal = vi.fn(async () => undefined);
  const mounted = render(SoundcheckSettings, {
    props: {
      session,
      configClient: config.client(),
      configSnapshot: configFixture.snapshot as unknown as ConfigOperationsSnapshot,
      reveal,
    },
  });

  await mounted.findByTestId("soundcheck-product");
  await waitFor(() => expect(reveal).toHaveBeenCalledOnce());
  expect(settings.authorityLoadedBeforeMutation()).toBe(true);
  expect(mounted.getByRole("button", { name: "Storage" })).toBeTruthy();
  expect(mounted.getByRole("button", { name: "Backups" })).toBeTruthy();
  expect(
    mounted.getByRole("button", { name: "Restore & Recovery" }),
  ).toBeTruthy();

  await fireEvent.click(
    mounted.getByRole("button", { name: "Use studio model" }),
  );
  await waitFor(() =>
    expect(settings.calls(SETTINGS_APPLY_COMMAND)).toBe(1),
  );
  await session.navigate({ pageId: "longhorn:storage" });
  expect(
    await mounted.findByRole("region", { name: "Active storage identity" }),
  ).toBeTruthy();
  await session.navigate({ pageId: "longhorn:backup" });
  expect(
    await mounted.findByRole("heading", { name: "Operational backups" }),
  ).toBeTruthy();

  await mounted.unmount();
  await session.stop();
});

test("executes an explicit restore plan through the shared page", async () => {
  const settings = new SettingsProofTransport("soundcheck");
  const config = new ConfigProofTransport();
  const session = settings.session();
  const mounted = render(SoundcheckSettings, {
    props: {
      session,
      configClient: config.client(),
      configSnapshot: configFixture.snapshot as unknown as ConfigOperationsSnapshot,
      reveal: async () => undefined,
    },
  });

  await mounted.findByTestId("soundcheck-product");
  await session.navigate({ pageId: "longhorn:restore" });
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Inspect archive" }),
  );
  const useArchive = await mounted.findAllByRole("radio", {
    name: "Use archive",
  });
  const keepCurrent = mounted.getAllByRole("radio", {
    name: "Keep current",
  });
  await fireEvent.click(useArchive[0]!);
  await fireEvent.click(keepCurrent[1]!);
  await fireEvent.click(keepCurrent[2]!);
  await fireEvent.click(
    mounted.getByRole("button", { name: "Review exact plan" }),
  );
  expect(config.publications).toBe(0);
  await fireEvent.click(
    await mounted.findByRole("button", {
      name: "Restore selected domains…",
    }),
  );
  await fireEvent.click(
    await mounted.findByRole("button", { name: "Publish restore" }),
  );

  expect(await mounted.findByText("Verified restore receipt")).toBeTruthy();
  expect(config.calls(CONFIG_RESTORE_INSPECT_COMMAND)).toBe(1);
  expect(config.calls(CONFIG_RESTORE_PLAN_COMMAND)).toBe(1);
  expect(config.calls(CONFIG_RESTORE_EXECUTE_COMMAND)).toBe(1);
  expect(config.publications).toBe(1);

  await mounted.unmount();
  await session.stop();
});
