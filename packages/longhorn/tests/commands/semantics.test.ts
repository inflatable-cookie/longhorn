import { describe, expect, test } from "bun:test";

import {
  resolveCommandKeyboard,
  searchCommands,
  shortcutsForCommand,
} from "../../src/commands/index.ts";
import { fixture } from "./support.ts";

describe("Rust and TypeScript command semantics", () => {
  test("share canonical search ranking", () => {
    for (const semantic of fixture.semantics.search) {
      expect(
        searchCommands(fixture.catalogue.commands, semantic.query),
      ).toEqual(semantic.hits);
    }
  });

  test("share platform shortcut labels and sources", () => {
    const keymap = fixture.snapshots[0]!;
    for (const semantic of fixture.semantics.shortcuts) {
      expect(
        shortcutsForCommand(
          keymap,
          semantic.commandId,
          semantic.platform,
        ),
      ).toEqual(semantic.shortcuts);
    }
  });

  test("share keyboard gates, capture, winners, and consumption", () => {
    for (const semantic of fixture.semantics.keyboard) {
      const resolution = resolveCommandKeyboard({
        platform: semantic.platform,
        input: semantic.input,
        contextPath: semantic.contextPath,
        mode: semantic.mode,
        bindings: semantic.bindings,
        commands: fixture.catalogue.commands,
        reserved:
          semantic.resolution.kind === "gated" &&
          semantic.resolution.gate === "reserved"
            ? () => true
            : undefined,
      });
      expect(resolution).toEqual(semantic.resolution);
      expect(
        resolution.kind === "captured" || resolution.kind === "resolved",
      ).toBe(semantic.consumed);
    }
  });
});
