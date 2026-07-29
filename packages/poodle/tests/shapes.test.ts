import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { LayoutDispatchResult } from "@longhorn/svelte/layout";

import WorkspaceShapeHarness from "./WorkspaceShapeHarness.svelte";
import {
  instance,
  loadShape,
  mountedBinding,
  resolvePanel,
  shapeDocument,
} from "./support.ts";

describe("mounted layout shapes", () => {
  it.each([
    ["nucleus", 5, "main"],
    ["loophole", 8, "primary"],
  ] as const)("mounts the %s region shape without host types", (name, count, occupied) => {
    const shape = loadShape(name);
    const document = shapeDocument(shape, {
      [occupied]: [instance("instance:a")],
    });
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      async () =>
        new Promise<LayoutDispatchResult>(() => {
          // No interaction in the shape proof.
        }),
    );
    const screen = render(WorkspaceShapeHarness, {
      props: {
        binding,
        regions: shape.schema.regions.map(({ id }) => id),
        resolvePanel,
      },
    });

    expect(screen.getAllByRole("region")).toHaveLength(count);
    expect(screen.getByRole("tab", { name: "A" })).toBeTruthy();
  });
});
