import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { SettingsSession } from "../../src/settings/poodle.ts";
import SettingsShellHarness from "./SettingsShellHarness.svelte";
import { FakeSettingsTransport, registry, requestIds } from "./support.ts";

function createSession(transport: FakeSettingsTransport): SettingsSession {
  return new SettingsSession({
    client: transport.client(),
    nextRequestId: requestIds("renderer-only-shell"),
  });
}

describe("renderer-only settings pages in the shell", () => {
  it("renders the page body for a page with no scopes or apply units", async () => {
    const transport = new FakeSettingsTransport();
    const value = registry();
    value.pages.push({
      ...structuredClone(value.pages[0]!),
      id: "bovine:updates",
      label: "Application updates",
      order: 99,
      anchors: [],
      requiredCapabilities: [],
      readableScopeIds: [],
      writableApplyUnitIds: [],
    });
    transport.registryValue = value;

    const mounted = render(SettingsShellHarness, {
      props: { session: createSession(transport) },
    });
    await mounted.findByTestId("consumer-page");

    const nav = document.querySelector('[aria-label="Settings pages"]');
    const item = Array.from(nav?.querySelectorAll("button, a, [role=button]") ?? []).find(
      (candidate) => candidate.textContent?.includes("Application updates"),
    );
    expect(item, "the renderer-only page is nav-reachable").toBeTruthy();
    await fireEvent.click(item!);

    await waitFor(() => {
      expect(document.querySelector('[data-page-id="bovine:updates"]')).not.toBeNull();
    });
  });
});
