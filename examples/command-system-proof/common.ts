import {
  CommandController,
  type CommandAvailabilitySnapshot,
  type CommandCatalogueSnapshot,
  type CommandDiscoveryRecord,
  type CommandEffectiveBinding,
  type CommandExecutionIntent,
  type CommandExecutionOutcome,
  type CommandKeymapLoadOutcome,
  type CommandKeymapPort,
  type CommandKeymapSnapshot,
  type CommandPorts,
  type CommandSnapshotPort,
  type CommandUnlisten,
  type KeyboardEventLike,
} from "@inflatable-cookie/longhorn-commands";

export type CommandShape = "jetstream" | "loophole";

const visibility = {
  hidden: false,
  palette: true,
  menu: true,
  shortcut: true,
  settings: true,
  help: true,
} as const;

function command(
  id: string,
  label: string,
  context: string,
  textInputPolicy: "allowed" | "blocked",
): CommandDiscoveryRecord & { readonly proofContext: string } {
  return {
    id,
    label,
    description: `${label} proof command`,
    categoryPath: [id.split(":")[1]!.split(".")[0]!],
    keywords: [label.toLowerCase()],
    icon: null,
    visibility,
    textInputPolicy,
    arguments: { shape: "none" },
    proofContext: context,
  };
}

const shapeCommands: Record<CommandShape, readonly ReturnType<typeof command>[]> = {
  jetstream: [
    command("jetstream:file.open", "Open File", "global", "allowed"),
  ],
  loophole: [
    command("loophole:editor.quantize", "Quantize", "editor", "blocked"),
    command("loophole:panel.close", "Close Panel", "panel", "allowed"),
    command("loophole:transport.play", "Play", "project", "blocked"),
  ],
};

const shapeContexts: Record<CommandShape, readonly string[]> = {
  jetstream: ["global"],
  loophole: ["global", "project", "surface", "region", "panel"],
};

const shapeCodes: Record<CommandShape, readonly string[]> = {
  jetstream: ["KeyO"],
  loophole: ["KeyQ", "KeyW", "Space"],
};

export class SnapshotPort<Snapshot> implements CommandSnapshotPort<Snapshot> {
  value: Snapshot;
  readonly loads: Array<Promise<Snapshot>> = [];
  readonly listeners = new Set<() => void>();
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

  listen(listener: () => void): CommandUnlisten {
    this.listeners.add(listener);
    return () => {
      if (this.listeners.delete(listener)) this.unlistenCount += 1;
    };
  }
}

export class KeyboardTarget {
  readonly listeners = new Set<(event: KeyboardEventLike) => void>();

  addEventListener(
    _type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void {
    this.listeners.add(listener);
  }

  removeEventListener(
    _type: "keydown",
    listener: (event: KeyboardEventLike) => void,
  ): void {
    this.listeners.delete(listener);
  }

  dispatch(event: KeyboardEventLike): void {
    for (const listener of this.listeners) listener(event);
  }
}

export function catalogue(shape: CommandShape): CommandCatalogueSnapshot {
  return {
    protocolVersion: 1,
    registryGeneration: 1,
    registryDigest: shape === "jetstream" ? "1".repeat(64) : "2".repeat(64),
    commands: shapeCommands[shape].map(({ proofContext: _, ...record }) => record),
    presets: [{ id: `${shape}:default`, version: 1 }],
  };
}

export function keymap(shape: CommandShape): CommandKeymapSnapshot {
  const currentCatalogue = catalogue(shape);
  const bindings = shapeCommands[shape].map(
    (record, index): CommandEffectiveBinding => ({
      id: `${shape}:binding.${index + 1}`,
      source: {
        kind: "preset",
        presetId: `${shape}:default`,
        presetVersion: 1,
      },
      platform: "any",
      trigger: {
        code: shapeCodes[shape][index]!,
        modifiers: {
          primary: true,
          control: false,
          alt: false,
          shift: false,
          meta: false,
        },
      },
      contextId: record.proofContext,
      invocation: { commandId: record.id, arguments: {} },
    }),
  );
  return {
    protocolVersion: 1,
    registryGeneration: currentCatalogue.registryGeneration,
    registryDigest: currentCatalogue.registryDigest,
    state: {
      revision: 1,
      activePresetId: `${shape}:default`,
      overrides: [],
    },
    activePresetVersion: 1,
    bindings,
    conflicts: [],
    origin: { kind: "default" },
    diagnostics: [],
  };
}

export function availability(
  shape: CommandShape,
  contextRevision = 1,
): CommandAvailabilitySnapshot {
  return {
    registryGeneration: 1,
    contextRevision,
    records: catalogue(shape).commands.map(({ id }) => ({
      commandId: id,
      availability: { state: "available", reason: null },
    })),
  };
}

export interface CommandHarness {
  readonly controller: CommandController;
  readonly catalogue: SnapshotPort<CommandCatalogueSnapshot>;
  readonly keymap: SnapshotPort<CommandKeymapLoadOutcome>;
  readonly availability: SnapshotPort<CommandAvailabilitySnapshot>;
  readonly executions: CommandExecutionIntent[];
  readonly operations: Array<
    | { readonly kind: "local"; readonly action: "close-panel" }
    | { readonly kind: "typed-domain"; readonly operation: "transport.play" }
    | { readonly kind: "typed-domain"; readonly operation: "editor.quantize" }
    | { readonly kind: "local"; readonly action: "open-file" }
  >;
}

export function createHarness(shape: CommandShape): CommandHarness {
  const cataloguePort = new SnapshotPort(catalogue(shape));
  const keymapPort = new SnapshotPort<CommandKeymapLoadOutcome>({
    status: "loaded",
    snapshot: keymap(shape),
  });
  const availabilityPort = new SnapshotPort(availability(shape));
  const executions: CommandExecutionIntent[] = [];
  const operations: CommandHarness["operations"][number][] = [];
  let request = 0;
  const ports: CommandPorts = {
    catalogue: cataloguePort,
    keymap: keymapPort,
    availability: availabilityPort,
    executor: {
      async execute(intent): Promise<CommandExecutionOutcome> {
        executions.push(intent);
        switch (intent.invocation.commandId) {
          case "jetstream:file.open":
            operations.push({ kind: "local", action: "open-file" });
            break;
          case "loophole:panel.close":
            operations.push({ kind: "local", action: "close-panel" });
            break;
          case "loophole:transport.play":
            operations.push({
              kind: "typed-domain",
              operation: "transport.play",
            });
            break;
          case "loophole:editor.quantize":
            operations.push({
              kind: "typed-domain",
              operation: "editor.quantize",
            });
            break;
          default:
            return { status: "unknownCommand" };
        }
        return { status: "succeeded" };
      },
    },
    nextRequestId: () => `${shape}:request.${++request}`,
  };
  return {
    controller: new CommandController({
      ports,
      platform: "macOs",
    }),
    catalogue: cataloguePort,
    keymap: keymapPort,
    availability: availabilityPort,
    executions,
    operations,
  };
}

export function contextPath(shape: CommandShape): readonly string[] {
  return shapeContexts[shape];
}

export function keyboardEvent(
  code: string,
  target?: unknown,
): KeyboardEventLike & {
  readonly counters: { prevented: number; stopped: number };
} {
  const counters = { prevented: 0, stopped: 0 };
  return {
    code,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: true,
    repeat: false,
    target,
    preventDefault: () => {
      counters.prevented += 1;
    },
    stopPropagation: () => {
      counters.stopped += 1;
    },
    counters,
  };
}
