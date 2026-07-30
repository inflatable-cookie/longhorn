import { render, waitFor } from "@testing-library/svelte";
import { expect, test, vi } from "vitest";

import NucleusSettings from "./NucleusSettings.svelte";
import { SettingsProofTransport } from "./settings-proof.ts";

test("omits absent Surface and backend modules without dead navigation", async () => {
  const transport = new SettingsProofTransport("nucleus");
  const session = transport.session();
  const reveal = vi.fn(async () => undefined);
  const mounted = render(NucleusSettings, { props: { session, reveal } });

  await mounted.findByTestId("nucleus-general");
  await waitFor(() => expect(reveal).toHaveBeenCalledOnce());
  expect(mounted.queryByText("Surfaces")).toBeNull();
  expect(mounted.queryByText("Server")).toBeNull();
  expect(transport.registryValue.pages).toHaveLength(1);

  await mounted.unmount();
  await session.stop();
  expect(transport.activeListenerCount()).toBe(0);
});
