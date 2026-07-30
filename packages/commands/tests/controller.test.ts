import { describe, expect, test } from "bun:test";

import {
  CommandController,
  routeCommandKeyboardEvent,
  type CommandAvailabilitySnapshot,
  type CommandExecutionIntent,
  type CommandExecutionOutcome,
  type CommandKeymapPort,
  type CommandKeymapLoadOutcome,
  type CommandPorts,
  type CommandSnapshotPort,
  type CommandUnlisten,
} from "../src/index.ts";
import { availability, fixture } from "./support.ts";

class SnapshotPort<Snapshot> implements CommandSnapshotPort<Snapshot> {
  value: Snapshot;
  loads: Array<Promise<Snapshot>> = [];
  pendingListen: Promise<void> | undefined;
  listeners = new Set<() => void>();
  unlistenCount = 0;
  preview?: CommandKeymapPort["preview"];
  commit?: CommandKeymapPort["commit"];
  reset?: CommandKeymapPort["reset"];

  constructor(value: Snapshot) {
    this.value = value;
  }

  load(): Promise<Snapshot> {
    return this.loads.shift() ?? Promise.resolve(this.value);
  }

  listen(listener: () => void): CommandUnlisten | Promise<CommandUnlisten> {
    this.listeners.add(listener);
    const dispose = () => {
      if (this.listeners.delete(listener)) this.unlistenCount += 1;
    };
    return this.pendingListen?.then(() => dispose) ?? dispose;
  }
}

function harness(options?: {
  availability?: CommandAvailabilitySnapshot;
  keymap?: CommandKeymapLoadOutcome;
  search?: ConstructorParameters<typeof CommandController>[0]["search"];
}) {
  const catalogue = new SnapshotPort(fixture.catalogue);
  const keymap = new SnapshotPort(
    options?.keymap ?? fixture.loadOutcomes[0]!,
  );
  const currentAvailability = new SnapshotPort(
    options?.availability ?? availability(),
  );
  const executions: CommandExecutionIntent[] = [];
  let request = 0;
  const ports: CommandPorts = {
    catalogue,
    keymap,
    availability: currentAvailability,
    executor: {
      async execute(intent): Promise<CommandExecutionOutcome> {
        executions.push(intent);
        return { status: "succeeded" };
      },
    },
    nextRequestId: () => `request:${++request}`,
  };
  const controller = new CommandController({
    ports,
    platform: "macOs",
    search: options?.search,
  });
  return {
    controller,
    catalogue,
    keymap,
    availability: currentAvailability,
    executions,
  };
}

describe("CommandController", () => {
  test("joins checked sources and routes palette and keyboard through one executor", async () => {
    const state = harness();
    await state.controller.start();
    expect(state.controller.status.kind).toBe("ready");

    await state.controller.dispatch({
      commandId: "app:open",
      source: "palette",
    });
    let prevented = 0;
    let stopped = 0;
    const routed = routeCommandKeyboardEvent(
      {
        code: "KeyO",
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
        metaKey: true,
        repeat: false,
        preventDefault: () => {
          prevented += 1;
        },
        stopPropagation: () => {
          stopped += 1;
        },
      },
      {
        platform: "macOs",
        contextPath: ["global"],
        mode: "dispatch",
        bindings: state.controller.model!.keymap.bindings,
        commands: state.controller.model!.catalogue.commands,
        dispatcher: state.controller,
      },
    );
    await routed.execution;

    expect(routed.consumed).toBeTrue();
    expect(prevented).toBe(1);
    expect(stopped).toBe(1);
    expect(state.executions.map(({ source }) => source)).toEqual([
      "palette",
      "keyboard",
    ]);
    expect(state.executions.every((intent) => !("transport" in intent))).toBeTrue();
    await state.controller.stop();
  });

  test("does not consume gated, conflicted, or unbound browser events", async () => {
    const state = harness();
    await state.controller.start();
    for (const semantic of fixture.semantics.keyboard.filter(
      ({ consumed }) => !consumed,
    )) {
      let consumed = 0;
      const routed = routeCommandKeyboardEvent(
        {
          code: semantic.input.chord.code,
          ctrlKey: semantic.input.chord.modifiers.control,
          altKey: semantic.input.chord.modifiers.alt,
          shiftKey: semantic.input.chord.modifiers.shift,
          metaKey: semantic.input.chord.modifiers.meta,
          repeat: semantic.input.repeat,
          isComposing: semantic.input.composing,
          target: semantic.input.editableText
            ? { tagName: "INPUT" }
            : undefined,
          preventDefault: () => {
            consumed += 1;
          },
          stopPropagation: () => {
            consumed += 1;
          },
        },
        {
          platform: semantic.platform,
          contextPath: semantic.contextPath,
          mode: semantic.mode,
          bindings: semantic.bindings,
          commands: state.controller.model!.catalogue.commands,
          dispatcher: state.controller,
          reserved:
            semantic.resolution.kind === "gated" &&
            semantic.resolution.gate === "reserved"
              ? () => true
              : undefined,
        },
      );
      expect(routed.consumed).toBeFalse();
      expect(consumed).toBe(0);
    }
    await state.controller.stop();
  });

  test("rejects stale search completion", async () => {
    const searches = new Map<
      string,
      (value: typeof fixture.semantics.search[0]["hits"]) => void
    >();
    const state = harness({
      search: async (records, query) => {
        if (query.length === 0) return searchAll(records);
        return new Promise((resolve) => {
          searches.set(query, resolve);
        });
      },
    });
    await state.controller.start();
    const first = state.controller.search("open");
    const second = state.controller.search("save");
    searches.get("save")!(fixture.semantics.search[2]!.hits);
    await second;
    searches.get("open")!(fixture.semantics.search[1]!.hits);
    await first;
    expect(state.controller.searchHits.map(({ record }) => record.id)).toEqual([
      "app:save",
    ]);
    await state.controller.stop();
  });

  test("rejects an older availability refresh", async () => {
    const state = harness();
    await state.controller.start();
    let release!: (value: CommandAvailabilitySnapshot) => void;
    state.availability.loads.push(
      new Promise((resolve) => {
        release = resolve;
      }),
      Promise.resolve(availability(3)),
    );
    const older = state.controller.refresh();
    const newer = state.controller.refresh();
    await newer;
    release(availability(2));
    await older;
    expect(state.controller.model?.availability.contextRevision).toBe(3);
    await state.controller.stop();
  });

  test("does not regress an installed authority revision", async () => {
    const state = harness({ availability: availability(4) });
    await state.controller.start();
    state.availability.value = availability(3);
    await state.controller.refresh();
    expect(state.controller.model?.availability.contextRevision).toBe(4);
    await state.controller.stop();
  });

  test("keeps a dirty draft when preview returns fresh stale authority", async () => {
    const state = harness();
    let commits = 0;
    state.keymap.preview = async () => fixture.previewResults[1]!;
    state.keymap.commit = async () => {
      commits += 1;
      return fixture.mutationResults[0]!;
    };
    await state.controller.start();
    const patch = fixture.requests.preview as {
      patch: Parameters<typeof state.controller.stageKeymapPatch>[0];
    };
    state.controller.stageKeymapPatch(patch.patch);
    expect(state.controller.dirty).toBeTrue();
    expect(await state.controller.applyKeymapDraft()).toBeFalse();
    expect(state.controller.mutation.kind).toBe("conflict");
    expect(state.controller.dirty).toBeTrue();
    expect(commits).toBe(0);
    await state.controller.stop();
  });

  test("keeps recovery and unavailable load postures explicit", async () => {
    const recovery = harness({ keymap: fixture.loadOutcomes[1]! });
    await recovery.controller.start();
    expect(recovery.controller.status.kind).toBe("recovery");
    expect(recovery.controller.model).toBeUndefined();
    await recovery.controller.stop();

    const unavailable = harness({ keymap: fixture.loadOutcomes[2]! });
    await unavailable.controller.start();
    expect(unavailable.controller.status.kind).toBe("unavailable");
    expect(unavailable.controller.model).toBeUndefined();
    await unavailable.controller.stop();
  });

  test("disposes listeners exactly once across repeated starts", async () => {
    const state = harness();
    for (let cycle = 0; cycle < 2; cycle += 1) {
      await state.controller.start();
      expect(state.catalogue.listeners.size).toBe(1);
      await state.controller.stop();
      expect(state.catalogue.listeners.size).toBe(0);
    }
    expect(state.catalogue.unlistenCount).toBe(2);
    expect(state.keymap.unlistenCount).toBe(2);
    expect(state.availability.unlistenCount).toBe(2);
  });

  test("disposes an async listener that resolves after stop", async () => {
    const state = harness();
    let release!: () => void;
    state.catalogue.pendingListen = new Promise((resolve) => {
      release = resolve;
    });
    const starting = state.controller.start();
    await Promise.resolve();
    await state.controller.stop();
    expect(state.controller.status.kind).toBe("idle");
    release();
    await starting;
    expect(state.catalogue.listeners.size).toBe(0);
    expect(state.catalogue.unlistenCount).toBe(1);
    expect(state.keymap.listeners.size).toBe(0);
    expect(state.availability.listeners.size).toBe(0);
  });
});

function searchAll(
  records: typeof fixture.catalogue.commands,
): typeof fixture.semantics.search[0]["hits"] {
  return records.map((record) => ({ record, score: 0 }));
}
