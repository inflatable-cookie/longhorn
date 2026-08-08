import { describe, expect, test } from "bun:test";

import {
  SettingsDeepLinkResolutionError,
  projectSettingsRegistry,
  resolveSettingsDeepLink,
  searchSettingsRegistry,
} from "../../src/settings/index.ts";
import { registry } from "./support.ts";

describe("settings registry projection", () => {
  test("projects registered order without adding empty navigation", () => {
    const projection = projectSettingsRegistry(registry());
    expect(projection.modules.map(({ module }) => module.id)).toEqual([
      "app:module",
    ]);
    expect(
      projection.modules[0]!.sections.map(({ section }) => section.id),
    ).toEqual(["app:general"]);
    expect(projection.pages.map((page) => page.id)).toEqual(["app:audio"]);
  });

  test("searches only labels and registered keywords in stable order", () => {
    expect(
      searchSettingsRegistry(registry(), "OUTPUT").map((result) => [
        result.kind,
        result.page.id,
        result.anchor?.id,
      ]),
    ).toEqual([
      ["page", "app:audio", undefined],
      ["anchor", "app:audio", "app:output"],
    ]);
    expect(searchSettingsRegistry(registry(), "audio")).toHaveLength(1);
    expect(searchSettingsRegistry(registry(), "selected")).toEqual([]);
  });

  test("resolves structural page and anchor links without a DOM", () => {
    expect(
      resolveSettingsDeepLink(registry(), {
        pageId: "app:audio",
        anchorId: "app:output",
      }).anchor?.label,
    ).toBe("Output device");
    expect(() =>
      resolveSettingsDeepLink(registry(), {
        pageId: "app:missing",
      }),
    ).toThrow(SettingsDeepLinkResolutionError);
  });
});
