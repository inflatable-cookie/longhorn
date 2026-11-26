import { describe, expect, test } from "bun:test";

import {
  CommandProjectionConsistencyError,
  projectCommandSurfaces,
  resolveCommandKeyboard,
  searchCommands,
  shortcutsForCommand,
} from "../../src/commands/index.ts";
import { availability, fixture } from "./support.ts";

describe("Rust and TypeScript command semantics", () => {
  test("share canonical search ranking", () => {
    for (const semantic of fixture.semantics.search) {
      expect(searchCommands(fixture.catalogue.commands, semantic.query)).toEqual(semantic.hits);
    }
  });

  test("share platform shortcut labels and sources", () => {
    const keymap = fixture.snapshots[0]!;
    for (const semantic of fixture.semantics.shortcuts) {
      expect(shortcutsForCommand(keymap, semantic.commandId, semantic.platform)).toEqual(
        semantic.shortcuts,
      );
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
          semantic.resolution.kind === "gated" && semantic.resolution.gate === "reserved"
            ? () => true
            : undefined,
      });
      expect(resolution).toEqual(semantic.resolution);
      expect(resolution.kind === "captured" || resolution.kind === "resolved").toBe(
        semantic.consumed,
      );
    }
  });

  test("orders equal-context binding IDs by UTF-8 order", () => {
    const scenario = fixture.semantics.keyboard.find(
      ({ resolution }) => resolution.kind === "resolved",
    )!;
    const binding = scenario.bindings[0]!;
    const firstId = "base:e\u0301";
    const resolution = resolveCommandKeyboard({
      ...scenario,
      bindings: [
        { ...binding, id: "base:é" },
        { ...binding, id: firstId },
      ],
      commands: fixture.catalogue.commands,
    });

    expect(resolution.kind).toBe("resolved");
    if (resolution.kind === "resolved") {
      expect(resolution.candidates[0]?.bindingId).toBe(firstId);
      expect(resolution.candidates[0]?.disposition.kind).toBe("winner");
    }
  });

  test("deduplicates equal invocations whose argument names collate equally", () => {
    const scenario = fixture.semantics.keyboard.find(
      ({ resolution }) => resolution.kind === "resolved",
    )!;
    const binding = scenario.bindings[0]!;
    const value = { kind: "boolean", value: true } as const;
    const resolution = resolveCommandKeyboard({
      ...scenario,
      bindings: [
        {
          ...binding,
          id: "base:first",
          invocation: { ...binding.invocation, arguments: { é: value, "e\u0301": value } },
        },
        {
          ...binding,
          id: "base:second",
          invocation: { ...binding.invocation, arguments: { "e\u0301": value, é: value } },
        },
      ],
      commands: fixture.catalogue.commands,
    });

    expect(resolution.kind).toBe("resolved");
  });

  test("refuses to project a command with missing availability", () => {
    const catalogue = fixture.catalogue;
    const state = availability();
    expect(() =>
      projectCommandSurfaces(
        {
          catalogue,
          keymap: fixture.snapshots[0]!,
          availability: { ...state, records: [] },
        },
        "macOs",
      ),
    ).toThrow(CommandProjectionConsistencyError);
  });
});
